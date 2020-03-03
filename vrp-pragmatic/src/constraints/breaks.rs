#[cfg(test)]
#[path = "../../tests/unit/constraints/breaks_test.rs"]
mod breaks_test;

use crate::constraints::*;
use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::common::{Cost, ValueDimension};
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::Activity;

/// Implements break functionality with variable location and time.
/// NOTE known issue: rescheduling departure might affect break with time offset.
pub struct BreakModule {
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
    /// Controls whether break should be considered as unassigned job
    demote_breaks_from_unassigned: bool,
}

impl BreakModule {
    pub fn new(code: i32, extra_break_cost: Option<Cost>, demote_breaks_from_unassigned: bool) -> Self {
        Self {
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(BreakHardRouteConstraint { code })),
                ConstraintVariant::HardActivity(Arc::new(BreakHardActivityConstraint { code })),
                ConstraintVariant::SoftRoute(Arc::new(BreakSoftRouteConstraint { extra_break_cost })),
            ],
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

struct BreakHardActivityConstraint {
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
}

impl HardActivityConstraint for BreakHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        match as_break_job(&activity_ctx.target) {
            Some(_) if activity_ctx.prev.job.is_none() => self.stop(),
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
/// * break is assigned right after departure
fn remove_orphan_breaks(ctx: &mut SolutionContext) {
    let breaks_set = ctx.routes.iter_mut().fold(HashSet::new(), |mut acc, rc: &mut RouteContext| {
        // NOTE assume that first activity is never break (should be always departure)
        let (_, breaks_set) = (0..).zip(rc.route.tour.all_activities()).fold(
            (0, HashSet::new()),
            |(prev, mut breaks), (idx, activity)| {
                let current = activity.place.location;

                if let Some(break_job) = as_break_job(activity) {
                    // NOTE break should have location defined for all places or for none of them
                    let location_count = break_job.places.iter().map(|p| p.location.is_some()).count();
                    assert!(location_count == 0 || location_count == break_job.places.len());

                    let is_orphan = prev != current && break_job.places.first().and_then(|p| p.location).is_none();
                    let is_dummy = idx == 1;

                    if is_orphan || is_dummy {
                        // NOTE remove break with removed job location
                        breaks.insert(Job::Single(activity.job.as_ref().unwrap().clone()));
                    }
                }

                (current, breaks)
            },
        );

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

fn is_time(rc: &RouteContext, break_job: &Arc<Single>) -> bool {
    let departure = rc.route.tour.start().unwrap().schedule.departure;
    let arrival = rc.route.tour.end().map_or(0., |end| end.schedule.arrival);

    break_job
        .places
        .first()
        .unwrap()
        .times
        .iter()
        .map(|span| span.to_time_window(departure))
        .any(|tw| tw.start < arrival)
}

//endregion
