use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Route;
use hashbrown::HashSet;
use rosomaxa::prelude::Objective;
use std::iter::once;
use std::slice::Iter;
use std::sync::Arc;

/// Creates a feature with multi trip functionality.
pub fn create_multi_trip_feature(
    name: &str,
    code: ViolationCode,
    state_keys: &[StateKey],
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
) -> Result<Feature, GenericError> {
    let state_keys = match route_intervals.get_interval_key() {
        Some(key) if !state_keys.contains(&key) => state_keys.iter().copied().chain(once(key)).collect(),
        _ => state_keys.to_vec(),
    };

    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(MultiTripConstraint::new(code, route_intervals.clone()))
        .with_objective(MultiTripObjective::new(route_intervals.clone()))
        .with_state(MultiTripState::new(code, state_keys, route_intervals))
        .build()
}

struct MultiTripConstraint {
    code: ViolationCode,
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
}

impl FeatureConstraint for MultiTripConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                if self.route_intervals.is_marker_job(job) {
                    return if self.route_intervals.is_marker_assignable(route_ctx.route(), job) {
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
                    .map_or(false, |job| self.route_intervals.is_marker_job(&Job::Single(job.clone())))
                {
                    // NOTE insert marker job in route only as last
                    let is_first = activity_ctx.prev.job.is_none();
                    let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

                    return if is_first || is_not_last {
                        ConstraintViolation::skip(self.code)
                    } else {
                        ConstraintViolation::success()
                    };
                };
            }
        }

        self.route_intervals.evaluate(move_ctx)
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        if once(&source).chain(once(&candidate)).any(|job| self.route_intervals.is_marker_job(job)) {
            return Err(self.code);
        }

        self.route_intervals.merge(source, candidate)
    }
}

impl MultiTripConstraint {
    fn new(code: ViolationCode, route_intervals: Arc<dyn RouteIntervals + Send + Sync>) -> Self {
        Self { code, route_intervals }
    }
}

struct MultiTripObjective {
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
}

impl MultiTripObjective {
    pub fn new(route_intervals: Arc<dyn RouteIntervals + Send + Sync>) -> Self {
        Self { route_intervals }
    }

    fn estimate_job(&self, job: &Job) -> Cost {
        if self.route_intervals.is_marker_job(job) {
            -1.
        } else {
            0.
        }
    }
}

impl Objective for MultiTripObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs())
            .map(|job| self.estimate_job(job))
            .sum()
    }
}

impl FeatureObjective for MultiTripObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { job, .. } => self.estimate_job(job),
            MoveContext::Activity { .. } => 0.,
        }
    }
}

struct MultiTripState {
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    state_keys: Vec<StateKey>,
    code: ViolationCode,
}

impl MultiTripState {
    pub fn new(
        code: ViolationCode,
        state_keys: Vec<StateKey>,
        route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    ) -> Self {
        let context_transition = Box::new(ConcreteJobContextTransition {
            remove_required: {
                let route_intervals = route_intervals.clone();
                move |_, _, job| route_intervals.is_marker_job(job)
            },
            promote_required: |_, _, _| false,
            remove_locked: |_, _, _| false,
            promote_locked: {
                let route_intervals = route_intervals.clone();
                move |_, _, job| route_intervals.is_marker_job(job)
            },
        });

        Self { route_intervals, context_transition, state_keys, code }
    }

    fn filter_markers<'a>(&'a self, route: &'a Route, jobs: &'a [Job]) -> impl Iterator<Item = Job> + 'a + Send + Sync {
        jobs.iter().filter(|job| self.route_intervals.is_marker_assignable(route, job)).cloned()
    }
}

impl FeatureState for MultiTripState {
    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }

    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());

        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
        if self.route_intervals.is_marker_job(job) {
            // move all unassigned marker jobs back to ignored
            let jobs = self.filter_markers(route_ctx.route(), &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.unassigned.retain(|job, _| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs.into_iter());
            // NOTE reevaluate insertion of unassigned due to multi-trip constraint jobs
            solution_ctx.unassigned.iter_mut().for_each(|pair| match pair.1 {
                UnassignmentInfo::Simple(code) if *code == self.code => {
                    *pair.1 = UnassignmentInfo::Unknown;
                }
                _ => {}
            });
        } else if self.route_intervals.is_new_interval_needed(route_ctx) {
            // move all marker jobs for this shift to required
            let jobs = self
                .filter_markers(route_ctx.route(), &solution_ctx.ignored)
                .chain(self.filter_markers(route_ctx.route(), &solution_ctx.required))
                .collect::<HashSet<_>>();

            solution_ctx.ignored.retain(|job| !jobs.contains(job));
            solution_ctx.locked.extend(jobs.iter().cloned());
            solution_ctx.required.extend(jobs.into_iter());
        }
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(interval_key) = self.route_intervals.get_interval_key() {
            let (route, state) = route_ctx.as_mut();
            let intervals = get_route_intervals(route, |a| {
                a.job.as_ref().map_or(false, |job| self.route_intervals.is_marker_job(&Job::Single(job.clone())))
            });

            state.put_route_state(interval_key, intervals);
        }
        self.route_intervals.update_route_intervals(route_ctx);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, self.context_transition.as_ref());

        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            self.accept_route_state(route_ctx);
        });

        self.route_intervals.update_solution_intervals(solution_ctx);
    }
}
