#[cfg(test)]
#[path = "../../tests/unit/constraints/breaks_test.rs"]
mod breaks_test;

use crate::constraints::*;
use std::collections::HashSet;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::common::{Cost, ValueDimension};
use vrp_core::models::problem::{ActivityCost, Job, Single, TransportCost};
use vrp_core::models::solution::Activity;

pub struct BreakModule {
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    /// Controls whether break should be considered as unassigned job
    demote_breaks_from_unassigned: bool,
}

impl BreakModule {
    pub fn new(
        activity: Arc<dyn ActivityCost + Send + Sync>,
        transport: Arc<dyn TransportCost + Send + Sync>,
        code: i32,
        extra_break_cost: Option<Cost>,
        demote_breaks_from_unassigned: bool,
    ) -> Self {
        Self {
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(BreakHardRouteConstraint { code })),
                ConstraintVariant::HardActivity(Arc::new(BreakHardActivityConstraint {
                    transport: transport.clone(),
                    code,
                })),
                ConstraintVariant::SoftRoute(Arc::new(BreakSoftRouteConstraint { extra_break_cost })),
            ],
            activity,
            transport,
            demote_breaks_from_unassigned,
        }
    }
}

impl ConstraintModule for BreakModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, _job: &Job) {
        self.accept_route_state(route_ctx);
        self.accept_solution_state(solution_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.conditional.accept_route_state(ctx);
        self.update_route_states(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);

        if ctx.required.is_empty() {
            remove_orphan_breaks(ctx);

            if self.demote_breaks_from_unassigned {
                demote_unassigned_breaks(ctx);
            }
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.conditional.state_keys()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl BreakModule {
    fn update_route_states(&self, ctx: &mut RouteContext) {
        let actor = ctx.route.actor.clone();
        let init = (
            actor.detail.time.end,
            actor.detail.end.unwrap_or_else(|| actor.detail.start.unwrap_or_else(|| panic!(OP_START_MSG))),
        );

        let (route, state) = ctx.as_mut();

        let start_departure = route.tour.start().unwrap().schedule.departure;

        route.tour.all_activities().rev().skip_while(|act| get_break_interval_from_activity(act).is_some()).fold(
            init,
            |acc, act| {
                if act.job.is_none() {
                    return acc;
                }

                let (end_time, prev_loc) = acc;
                let potential_latest = end_time
                    - self.transport.duration(actor.vehicle.profile, act.place.location, prev_loc, end_time)
                    - self.activity.duration(actor.as_ref(), act.deref(), end_time);

                let latest_arrival_time = as_break_job(act)
                    .and_then(|job| get_break_interval(job))
                    .map(|&interval| start_departure + interval.1)
                    .unwrap_or(act.place.time.end)
                    .min(potential_latest);

                // NOTE override LATEST_ARRIVAL_KEY set from transport constraint
                state.put_activity_state(LATEST_ARRIVAL_KEY, &act, latest_arrival_time);

                (latest_arrival_time, act.place.location)
            },
        );
    }
}

struct BreakHardActivityConstraint {
    transport: Arc<dyn TransportCost + Send + Sync>,
    code: i32,
}

/// Locks break jobs to specific vehicles.
struct BreakHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for BreakHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(single) = job.as_single() {
            if is_break_job(single) {
                let job = job.to_single();
                let vehicle_id = get_vehicle_id_from_job(&job).unwrap();
                let shift_index = get_shift_index(&job.dimens);

                return if !is_correct_vehicle(&ctx.route, vehicle_id, shift_index) {
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
    fn stop(&self) -> Option<ActivityConstraintViolation> {
        Some(ActivityConstraintViolation { code: self.code, stopped: false })
    }

    fn fail(&self) -> Option<ActivityConstraintViolation> {
        Some(ActivityConstraintViolation { code: self.code, stopped: true })
    }
}

impl HardActivityConstraint for BreakHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        match as_break_job(&activity_ctx.target) {
            Some(_) if activity_ctx.prev.job.is_none() => self.stop(),
            Some(break_job) => {
                if let Some(&interval) = get_break_interval(break_job) {
                    let arrival = activity_ctx.prev.schedule.departure
                        + self.transport.duration(
                            route_ctx.route.actor.vehicle.profile,
                            activity_ctx.prev.place.location,
                            activity_ctx.target.place.location,
                            activity_ctx.prev.schedule.departure,
                        );
                    let start_departure = route_ctx.route.tour.start().unwrap().schedule.departure;
                    if arrival > start_departure + interval.1 {
                        self.fail()
                    } else if arrival < start_departure + interval.0 {
                        self.stop()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Controls whether break is more preferable for insertion or not.
struct BreakSoftRouteConstraint {
    /// Allows to control whether break should be preferable for insertion
    extra_break_cost: Option<Cost>,
}

impl SoftRouteConstraint for BreakSoftRouteConstraint {
    fn estimate_job(&self, _ctx: &RouteContext, job: &Job) -> f64 {
        if let Some(cost) = self.extra_break_cost {
            if let Some(single) = job.as_single() {
                return if is_break_job(single) { cost } else { 0. };
            }
        }

        0.
    }
}

/// Promotes break jobs from required and ignored.
fn create_job_transition() -> Box<dyn JobContextTransition + Send + Sync> {
    Box::new(ConcreteJobContextTransition {
        remove_required: |ctx, job| !is_required_job(ctx, job, true),
        promote_required: |ctx, job| is_required_job(ctx, job, false),
        remove_locked: |_, _| false,
        promote_locked: |_, _| false,
    })
}

/// Mark job as ignored only if it has break type and vehicle id is not present in routes
fn is_required_job(ctx: &SolutionContext, job: &Job, default: bool) -> bool {
    match job {
        Job::Single(job) => {
            if is_break_job(job) {
                let vehicle_id = get_vehicle_id_from_job(job).unwrap();
                let shift_index = get_shift_index(&job.dimens);
                ctx.routes
                    .iter()
                    .any(move |rc| is_correct_vehicle(&rc.route, &vehicle_id, shift_index) && is_time(rc, job))
            } else {
                default
            }
        }
        Job::Multi(_) => default,
    }
}

/// Remove some breaks from required jobs as we don't want to consider breaks
/// as unassigned jobs if they are outside of vehicle's time window
fn demote_unassigned_breaks(ctx: &mut SolutionContext) {
    if ctx.unassigned.is_empty() {
        return;
    }

    // NOTE remove all breaks from list of unassigned jobs
    let breaks_set: HashSet<_> = ctx
        .unassigned
        .iter()
        .filter_map(|(job, _)| job.as_single().and_then(|single| get_vehicle_id_from_job(single).map(|_| job.clone())))
        .collect();

    ctx.unassigned.retain(|job, _| breaks_set.get(job).is_none());
    ctx.ignored.extend(breaks_set.into_iter());
}

/// Removes breaks which conditions are violated after ruin:
/// * break without location served separately when original job is removed, but break is kept.
/// * interval break outside of its interval
fn remove_orphan_breaks(ctx: &mut SolutionContext) {
    let breaks_set = ctx.routes.iter_mut().fold(HashSet::new(), |mut acc, rc: &mut RouteContext| {
        // NOTE assume that first activity is never break (should be always departure)
        let (_, breaks_set) =
            rc.route.tour.all_activities().fold((0, HashSet::new()), |(prev, mut breaks), activity| {
                let current = activity.place.location;

                if let Some(break_job) = as_break_job(activity) {
                    // NOTE break should have location defined for all places or for none of them
                    let location_count = break_job.places.iter().map(|p| p.location.is_some()).count();
                    assert!(location_count == 0 || location_count == break_job.places.len());

                    if prev != current && break_job.places.first().and_then(|p| p.location).is_none() {
                        // NOTE remove break with removed job location
                        breaks.insert(Job::Single(activity.job.as_ref().unwrap().clone()));
                    }

                    if let Some(interval) = get_break_interval(break_job) {
                        // NOTE remove interval breaks earlier their interval
                        if activity.schedule.arrival < rc.route.tour.start().unwrap().schedule.departure + interval.0 {
                            breaks.insert(Job::Single(activity.job.as_ref().unwrap().clone()));
                        }
                    }
                }

                (current, breaks)
            });

        breaks_set.iter().for_each(|break_job| {
            rc.route_mut().tour.remove(break_job);
        });

        acc.extend(breaks_set.into_iter());

        acc
    });

    ctx.required.extend(breaks_set.into_iter());
}

//region Helpers

fn is_break_job(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "break")
}

fn as_break_job(activity: &Activity) -> Option<&Arc<Single>> {
    as_single_job(activity, |job| is_break_job(job))
}

fn get_break_interval(job: &Arc<Single>) -> Option<&(f64, f64)> {
    job.dimens.get_value::<(f64, f64)>("interval")
}

fn get_break_interval_from_activity(activity: &Activity) -> Option<&(f64, f64)> {
    as_break_job(activity).and_then(|break_job| get_break_interval(break_job))
}

fn is_time(rc: &RouteContext, break_job: &Arc<Single>) -> bool {
    let tour = &rc.route.tour;
    let departure = tour.start().unwrap().schedule.departure;
    if let Some(&interval) = get_break_interval(break_job) {
        let tour_duration = tour.end().unwrap().schedule.arrival - departure;
        tour_duration > interval.0
    } else {
        let arrival = rc.route.tour.end().map_or(0., |end| end.schedule.arrival);
        let place = break_job.places.first().unwrap();

        place.times.iter().map(|span| span.to_time_window(departure)).any(|tw| tw.start < arrival)
    }
}

//endregion
