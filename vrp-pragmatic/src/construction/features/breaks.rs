//! A vehicle break features.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/breaks_test.rs"]
mod breaks_test;

use super::*;
use crate::construction::enablers::*;
use crate::construction::enablers::{BreakTie, JobTie};
use hashbrown::HashSet;
use std::iter::once;
use vrp_core::construction::enablers::*;
use vrp_core::models::common::{Cost, Schedule, TimeWindow};
use vrp_core::models::problem::Single;
use vrp_core::models::solution::Activity;
use vrp_core::rosomaxa::prelude::Objective;

/// Specifies break policy.
#[derive(Clone)]
pub enum BreakPolicy {
    /// Allows to skip break if actual tour schedule doesn't intersect with vehicle time window.
    SkipIfNoIntersection,
    /// Allows to skip break if vehicle arrives before break's time window end.
    SkipIfArrivalBeforeEnd,
}

/// Creates a feature to schedule an optional break. Here, optional means that break sometimes can
/// be skipped due to constraint violations or suboptimal search path in solution space.
pub fn create_optional_break_feature(name: &str, code: ViolationCode) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(OptionalBreakConstraint { code })
        .with_objective(OptionalBreakObjective {})
        .with_state(OptionalBreakState {
            context_transition: Box::new(ConcreteJobContextTransition {
                remove_required: |solution_ctx, route_index, job| {
                    !is_required_job(solution_ctx.routes.as_slice(), route_index, job, true)
                },
                promote_required: |solution_ctx, route_index, job| {
                    is_required_job(solution_ctx.routes.as_slice(), route_index, job, false)
                },
                remove_locked: |_, _, _| false,
                promote_locked: |_, _, _| false,
            }),
            state_keys: vec![code],
        })
        .build()
}

struct OptionalBreakConstraint {
    code: ViolationCode,
}

impl OptionalBreakConstraint {
    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        job.as_single()
            .filter(|single| is_break_single(single) && !is_single_belongs_to_route(route_ctx.route(), single))
            .and_then(|_| ConstraintViolation::fail(self.code))
    }

    fn evaluate_activity(&self, activity_ctx: &ActivityContext) -> Option<ConstraintViolation> {
        match as_break_job(activity_ctx.target) {
            Some(_) if activity_ctx.prev.job.is_none() => ConstraintViolation::skip(self.code),
            _ => None,
        }
    }
}

impl FeatureConstraint for OptionalBreakConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { activity_ctx, .. } => self.evaluate_activity(activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        let any_is_break = once(&source).chain(once(&candidate)).flat_map(|job| job.as_single()).any(is_break_single);

        if any_is_break {
            Err(self.code)
        } else {
            Ok(source)
        }
    }
}

struct OptionalBreakObjective {}

impl Objective for OptionalBreakObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs())
            .filter(|job| job.as_single().filter(|single| is_break_single(single)).is_some())
            .count() as f64
    }
}

impl FeatureObjective for OptionalBreakObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { job, .. } => {
                job.as_single().filter(|single| is_break_single(single)).map_or(Cost::default(), |_| 1.)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct OptionalBreakState {
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    state_keys: Vec<StateKey>,
}

impl FeatureState for OptionalBreakState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        process_conditional_jobs(solution_ctx, Some(route_index), self.context_transition.as_ref());
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, self.context_transition.as_ref());
        remove_invalid_breaks(solution_ctx);
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

/// Mark job as ignored only if it has break type and vehicle id is not present in routes
fn is_required_job(routes: &[RouteContext], route_index: Option<usize>, job: &Job, default: bool) -> bool {
    match job {
        Job::Single(job) => {
            if is_break_single(job) {
                if let Some(route_index) = route_index {
                    can_be_scheduled(routes.get(route_index).unwrap(), job)
                } else {
                    let vehicle_id = get_vehicle_id_from_job(job);
                    let shift_index = get_shift_index(&job.dimens);
                    routes.iter().any(move |route_ctx| {
                        is_correct_vehicle(route_ctx.route(), vehicle_id, shift_index)
                            && can_be_scheduled(route_ctx, job)
                    })
                }
            } else {
                default
            }
        }
        Job::Multi(_) => default,
    }
}

/// Removes breaks which conditions are violated after ruin:
/// * break without location served separately when original job is removed, but break is kept.
/// * break is defined by interval, but its time is violated. This might happen due to departure time rescheduling.
fn remove_invalid_breaks(ctx: &mut SolutionContext) {
    let breaks_to_remove = ctx
        .routes
        .iter()
        .flat_map(|route_ctx| {
            route_ctx
                .route()
                .tour
                .all_activities()
                .fold((0, HashSet::new()), |(prev, mut breaks), activity| {
                    let current = activity.place.location;

                    if let Some(break_single) = as_break_job(activity) {
                        let break_job = Job::Single(break_single.clone());
                        let is_locked = ctx.locked.contains(&break_job);

                        if !is_locked {
                            // NOTE break should have location defined for all places or for none of them
                            let location_count = break_single.places.iter().filter(|p| p.location.is_some()).count();
                            assert!(location_count == 0 || location_count == break_single.places.len());

                            let is_orphan =
                                prev != current && break_single.places.first().and_then(|p| p.location).is_none();
                            let is_not_on_time = !is_on_proper_time(route_ctx, break_single, &activity.schedule)
                                || !can_be_scheduled(route_ctx, break_single);
                            let is_ovrp_last =
                                route_ctx.route().tour.end().map_or(false, |end| std::ptr::eq(activity, end));

                            if is_orphan || is_not_on_time || is_ovrp_last {
                                // NOTE remove break with removed job location
                                breaks.insert(Job::Single(activity.job.as_ref().unwrap().clone()));
                            }
                        }
                    }

                    (current, breaks)
                })
                .1
                .into_iter()
        })
        .collect::<Vec<_>>();

    breaks_to_remove.iter().for_each(|break_job| {
        ctx.routes.iter_mut().filter(|route_ctx| route_ctx.route().tour.contains(break_job)).for_each(|route_ctx| {
            route_ctx.route_mut().tour.remove(break_job);
        })
    });

    ctx.unassigned.extend(breaks_to_remove.into_iter().map(|b| (b, UnassignmentInfo::Unknown)));

    // NOTE remove stale breaks from violation list
    ctx.ignored.extend(
        ctx.unassigned
            .drain_filter({
                let routes = ctx.routes.as_slice();
                move |job, _| !is_required_job(routes, None, job, true)
            })
            .map(|(job, _)| job),
    );
}

fn is_break_single(single: &Arc<Single>) -> bool {
    single.dimens.get_job_type().map_or(false, |t| t == "break")
}

fn as_break_job(activity: &Activity) -> Option<&Arc<Single>> {
    as_single_job(activity, is_break_single)
}

fn get_break_time_windows(break_job: &'_ Arc<Single>, departure: f64) -> impl Iterator<Item = TimeWindow> + '_ {
    break_job.places.first().unwrap().times.iter().map(move |span| span.to_time_window(departure))
}

/// Checks whether break can be scheduled in route.
fn can_be_scheduled(route_ctx: &RouteContext, break_job: &Arc<Single>) -> bool {
    let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
    let arrival = route_ctx.route().tour.end().map_or(0., |end| end.schedule.arrival);
    let tour_tw = TimeWindow::new(departure, arrival);
    let policy = break_job.dimens.get_break_policy().unwrap_or(BreakPolicy::SkipIfNoIntersection);

    get_break_time_windows(break_job, departure).any(|break_tw| match policy {
        BreakPolicy::SkipIfNoIntersection => break_tw.intersects(&tour_tw),
        BreakPolicy::SkipIfArrivalBeforeEnd => tour_tw.end > break_tw.end,
    })
}

/// Checks whether break is scheduled on time as its time can be invalid due to departure time optimizations.
fn is_on_proper_time(route_ctx: &RouteContext, break_job: &Arc<Single>, actual_schedule: &Schedule) -> bool {
    let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
    let actual_tw = TimeWindow::new(actual_schedule.arrival, actual_schedule.departure);

    get_break_time_windows(break_job, departure).any(|tw| tw.intersects(&actual_tw))
}
