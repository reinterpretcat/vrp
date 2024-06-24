//! Provides a skeleton to model multi trip functionality. It uses concept of marker job to reset
//! some constraint limitations once the marker job's activity is visited.

use crate::construction::enablers::*;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::models::*;
use rosomaxa::prelude::*;
use std::collections::HashSet;
use std::iter::once;
use std::sync::Arc;

/// Specifies multi trip extension behavior.
pub trait MultiTrip {
    /// Gets an actual route intervals.
    fn get_route_intervals(&self) -> &RouteIntervals;

    /// Gets an actual feature constraint which restricts a route.
    fn get_constraint(&self) -> &(dyn FeatureConstraint);

    /// Recalculates inner states for given route.
    fn recalculate_states(&self, route_ctx: &mut RouteContext);

    /// Provides the way to recover from inability of the solver to insert jobs.
    /// Returns true if some recovery actions were taken.
    fn try_recover(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool;
}

/// Marker insertion policy.
pub enum MarkerInsertionPolicy {
    /// Any position.
    Any,
    /// Only last position is allowed.
    Last,
}

/// Creates a feature with multi trip functionality.
pub fn create_multi_trip_feature(
    name: &str,
    violation_code: ViolationCode,
    policy: MarkerInsertionPolicy,
    multi_trip: Arc<dyn MultiTrip + Send + Sync>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(MultiTripConstraint::new(violation_code, policy, multi_trip.clone()))
        .with_state(MultiTripState::new(violation_code, multi_trip))
        .build()
}

struct MultiTripConstraint {
    code: ViolationCode,
    policy: MarkerInsertionPolicy,
    multi_trip: Arc<dyn MultiTrip + Send + Sync>,
}

impl FeatureConstraint for MultiTripConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        let intervals = self.multi_trip.get_route_intervals();
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                if intervals.is_marker_job(job) {
                    return if intervals.is_marker_assignable(route_ctx.route(), job) {
                        None
                    } else {
                        Some(ConstraintViolation { code: self.code, stopped: true })
                    };
                };
            }
            MoveContext::Activity { activity_ctx, .. } => {
                if activity_ctx
                    .target
                    .job
                    .as_ref()
                    .map_or(false, |job| intervals.is_marker_job(&Job::Single(job.clone())))
                {
                    match self.policy {
                        MarkerInsertionPolicy::Any => {}
                        MarkerInsertionPolicy::Last => {
                            let is_first = activity_ctx.prev.job.is_none();
                            let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

                            if is_first || is_not_last {
                                return ConstraintViolation::skip(self.code);
                            }
                        }
                    }
                }
            }
        }

        self.multi_trip.get_constraint().evaluate(move_ctx)
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        if once(&source).chain(once(&candidate)).any(|job| self.multi_trip.get_route_intervals().is_marker_job(job)) {
            return Err(self.code);
        }

        self.multi_trip.get_constraint().merge(source, candidate)
    }
}

impl MultiTripConstraint {
    fn new(code: ViolationCode, policy: MarkerInsertionPolicy, multi_trip: Arc<dyn MultiTrip + Send + Sync>) -> Self {
        Self { code, policy, multi_trip }
    }
}

struct MultiTripState {
    multi_trip: Arc<dyn MultiTrip + Send + Sync>,
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    code: ViolationCode,
}

impl MultiTripState {
    pub fn new(code: ViolationCode, multi_trip: Arc<dyn MultiTrip + Send + Sync>) -> Self {
        let context_transition = Box::new(ConcreteJobContextTransition {
            remove_required: {
                let multi_trip = multi_trip.clone();
                move |_, _, job| multi_trip.get_route_intervals().is_marker_job(job)
            },
            promote_required: |_, _, _| false,
            remove_locked: |_, _, _| false,
            promote_locked: {
                let multi_trip = multi_trip.clone();
                move |_, _, job| multi_trip.get_route_intervals().is_marker_job(job)
            },
        });

        Self { multi_trip, context_transition, code }
    }

    fn filter_markers<'a>(&'a self, route: &'a Route, jobs: &'a [Job]) -> impl Iterator<Item = Job> + 'a + Send + Sync {
        jobs.iter().filter(|job| self.multi_trip.get_route_intervals().is_marker_assignable(route, job)).cloned()
    }
}

impl FeatureState for MultiTripState {
    fn notify_failure(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool {
        self.multi_trip.try_recover(solution_ctx, route_indices, jobs)
    }

    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());

        let intervals = self.multi_trip.get_route_intervals();

        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
        if intervals.is_marker_job(job) {
            // move all unassigned marker jobs back to ignored
            let jobs = self.filter_markers(route_ctx.route(), &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.unassigned.retain(|job, _| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs);
            // NOTE reevaluate insertion of unassigned due to multi-trip constraint jobs
            solution_ctx.unassigned.iter_mut().for_each(|pair| match pair.1 {
                UnassignmentInfo::Simple(code) if *code == self.code => {
                    *pair.1 = UnassignmentInfo::Unknown;
                }
                _ => {}
            });
        } else if intervals.is_new_interval_needed(route_ctx) {
            // move all marker jobs for this shift to required
            let jobs = self
                .filter_markers(route_ctx.route(), &solution_ctx.ignored)
                .chain(self.filter_markers(route_ctx.route(), &solution_ctx.required))
                .collect::<HashSet<_>>();

            solution_ctx.ignored.retain(|job| !jobs.contains(job));
            solution_ctx.locked.extend(jobs.iter().cloned());
            solution_ctx.required.extend(jobs);
        }
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let route_intervals = self.multi_trip.get_route_intervals();

        if let Some(interval_fn) = route_intervals.get_interval_fn() {
            let (route, state) = route_ctx.as_mut();
            let intervals = get_route_intervals(route, |a| {
                a.job.as_ref().map_or(false, |job| route_intervals.is_marker_job(&Job::Single(job.clone())))
            });

            interval_fn.set_route_intervals(state, intervals);
        }

        self.multi_trip.recalculate_states(route_ctx);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, self.context_transition.as_ref());

        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            self.accept_route_state(route_ctx);
        });

        self.multi_trip.get_route_intervals().update_solution_intervals(solution_ctx);
    }
}
