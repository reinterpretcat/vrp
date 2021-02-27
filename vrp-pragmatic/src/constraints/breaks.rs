#[cfg(test)]
#[path = "../../tests/unit/constraints/breaks_test.rs"]
mod breaks_test;

use crate::constraints::*;
use hashbrown::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::common::{Schedule, TimeWindow, ValueDimension};
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::Activity;

/// Implements break functionality with variable location and time.
/// NOTE known issue: rescheduling departure might affect break with time offset.
pub struct BreakModule {
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl BreakModule {
    pub fn new(code: i32) -> Self {
        Self {
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(BreakHardRouteConstraint { code })),
                ConstraintVariant::HardActivity(Arc::new(BreakHardActivityConstraint { code })),
                ConstraintVariant::SoftRoute(Arc::new(BreakSoftRouteConstraint {})),
            ],
        }
    }
}

impl ConstraintModule for BreakModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.conditional.accept_insertion(solution_ctx, route_index, job);
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
        remove_invalid_breaks(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.conditional.state_keys()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct BreakHardActivityConstraint {
    code: i32,
}

/// Locks break jobs to specific vehicles.
struct BreakHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for BreakHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(single) = job.as_single() {
            if is_break_single(single) {
                return if !is_single_belongs_to_route(ctx, single) {
                    Some(RouteConstraintViolation { code: self.code })
                } else {
                    None
                };
            }
        }

        None
    }
}

impl BreakHardActivityConstraint {
    fn stop(&self) -> ActivityConstraintViolation {
        ActivityConstraintViolation { code: self.code, stopped: false }
    }
}

impl HardActivityConstraint for BreakHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        match as_break_job(&activity_ctx.target) {
            Some(_) if activity_ctx.prev.job.is_none() => Some(self.stop()),
            _ => None,
        }
    }
}

/// Controls whether break is more preferable for insertion or not.
struct BreakSoftRouteConstraint {}

impl SoftRouteConstraint for BreakSoftRouteConstraint {
    fn estimate_job(&self, solution_ctx: &SolutionContext, _: &RouteContext, job: &Job) -> f64 {
        if let Some(single) = job.as_single() {
            if is_break_single(single) {
                -solution_ctx.get_max_cost()
            } else {
                0.
            }
        } else {
            0.
        }
    }
}

/// Promotes break jobs from required and ignored.
fn create_job_transition() -> Box<dyn JobContextTransition + Send + Sync> {
    Box::new(ConcreteJobContextTransition {
        remove_required: |ctx, route_index, job| !is_required_job(ctx.routes.as_slice(), route_index, job, true),
        promote_required: |ctx, route_index, job| is_required_job(ctx.routes.as_slice(), route_index, job, false),
        remove_locked: |_, _, _| false,
        promote_locked: |_, _, _| false,
    })
}

/// Mark job as ignored only if it has break type and vehicle id is not present in routes
fn is_required_job(routes: &[RouteContext], route_index: Option<usize>, job: &Job, default: bool) -> bool {
    match job {
        Job::Single(job) => {
            if is_break_single(job) {
                if let Some(route_index) = route_index {
                    is_time(routes.get(route_index).unwrap(), job)
                } else {
                    let vehicle_id = get_vehicle_id_from_job(job).unwrap();
                    let shift_index = get_shift_index(&job.dimens);
                    routes
                        .iter()
                        .any(move |rc| is_correct_vehicle(&rc.route, &vehicle_id, shift_index) && is_time(rc, job))
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
        .flat_map(|rc| {
            rc.route
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
                            let is_not_on_time = !is_on_proper_time(rc, break_single, &activity.schedule);
                            let is_ovrp_last = rc.route.tour.end().map_or(false, |end| std::ptr::eq(activity, end));

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
        ctx.routes.iter_mut().filter(|route_ctx| route_ctx.route.tour.contains(break_job)).for_each(|route_ctx| {
            route_ctx.route_mut().tour.remove(break_job);
        })
    });

    ctx.unassigned.extend(breaks_to_remove.into_iter().map(|b| (b, 0)));

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

//region Helpers

fn is_break_single(single: &Arc<Single>) -> bool {
    single.dimens.get_value::<String>("type").map_or(false, |t| t == "break")
}

fn as_break_job(activity: &Activity) -> Option<&Arc<Single>> {
    as_single_job(activity, |job| is_break_single(job))
}

fn get_break_time_windows(break_job: &'_ Arc<Single>, departure: f64) -> impl Iterator<Item = TimeWindow> + '_ {
    break_job.places.first().unwrap().times.iter().map(move |span| span.to_time_window(departure))
}

fn is_time(rc: &RouteContext, break_job: &Arc<Single>) -> bool {
    let departure = rc.route.tour.start().unwrap().schedule.departure;
    let arrival = rc.route.tour.end().map_or(0., |end| end.schedule.arrival);
    let actual_shift_time = TimeWindow::new(departure, arrival);

    get_break_time_windows(break_job, departure).any(|tw| tw.intersects(&actual_shift_time))
}

fn is_on_proper_time(rc: &RouteContext, break_job: &Arc<Single>, actual_schedule: &Schedule) -> bool {
    let departure = rc.route.tour.start().unwrap().schedule.departure;
    let actual_tw = TimeWindow::new(actual_schedule.arrival, actual_schedule.departure);

    get_break_time_windows(break_job, departure).any(|tw| tw.intersects(&actual_tw))
}

//endregion
