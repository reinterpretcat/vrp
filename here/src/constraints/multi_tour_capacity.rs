use crate::constraints::{as_single_job, get_shift_index, get_vehicle_id_from_job, is_correct_vehicle};
use core::construction::constraints::*;
use core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use core::models::common::{Dimensions, IdDimension, ValueDimension};
use core::models::problem::{Job, Single};
use core::models::solution::Activity;
use std::iter::once;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

const MULTI_TOUR_KEY: i32 = 101;

pub struct MultiTourCapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    minimum: Capacity,
    state_keys: Vec<i32>,
    capacity_inner: CapacityConstraintModule<Capacity>,
    conditional_inner: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    MultiTourCapacityConstraintModule<Capacity>
{
    pub fn new(code: i32, minimum: Capacity) -> Self {
        let mut constraints = vec![
            ConstraintVariant::HardRoute(Arc::new(MultiTourHardRouteConstraint { code })),
            ConstraintVariant::HardActivity(Arc::new(MultiTourHardActivityConstraint { code })),
        ];
        constraints.extend(CapacityConstraintModule::<Capacity>::new(code).constraints());

        let capacity_inner = CapacityConstraintModule::new(code);
        let conditional_inner =
            ConditionalJobModule::new(Box::new(move |ctx, job| Self::is_required_job(ctx, job, &minimum)));

        Self {
            minimum,
            state_keys: capacity_inner.state_keys().chain(once(&MULTI_TOUR_KEY)).cloned().collect(),
            capacity_inner,
            conditional_inner,
            constraints,
        }
    }

    fn is_vehicle_full(rc: &RouteContext, minimum: &Capacity) -> bool {
        let tour = &rc.route.tour;
        let state = &rc.state;

        if let (Some(start), Some(end)) = (tour.start(), tour.end()) {
            let empty_capacity = Capacity::default();

            let max_capacity: Capacity = *rc.route.actor.vehicle.dimens.get_capacity().unwrap();
            let max_capacity = max_capacity - *minimum;

            let load = state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).unwrap_or_else(|| &empty_capacity);

            load > &max_capacity
        } else {
            false
        }
    }

    fn is_required_job(ctx: &SolutionContext, job: &Arc<Job>, minimum: &Capacity) -> bool {
        match job.as_ref() {
            Job::Single(job) => {
                if let Some(tour_index) = get_tour_index(job) {
                    let vehicle_id = get_vehicle_id_from_job(job).unwrap();
                    let shift_index = get_shift_index(&job.dimens);
                    ctx.routes
                        .iter()
                        .any(move |rc| is_correct_vehicle(rc, vehicle_id, shift_index) && is_time(rc, tour_index))
                } else {
                    true
                }
            }
            Job::Multi(_) => true,
        }
    }

    fn recalculate_states(ctx: &mut RouteContext) {
        let (route, state) = ctx.as_mut();

        let (_, starts) = (0_usize..).zip(route.tour.all_activities()).fold(
            (Capacity::default(), Vec::<(usize, usize, Capacity)>::default()),
            |(total, mut acc), (idx, a)| {
                let total = if as_multi_tour_job(a).is_some() {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
                    let end_idx = idx - 1;

                    acc.push((start_idx, end_idx, total));

                    Capacity::default()
                } else {
                    total
                        + CapacityConstraintModule::<Capacity>::get_demand(a)
                            .map_or(Capacity::default(), |d| d.delivery.0)
                };

                (total, acc)
            },
        );

        let last_multi_tour_index = starts.len() - 1;

        let ends = starts.iter().cloned().fold(vec![], |mut acc, (start_idx, end_idx, total)| {
            let (current, _) =
                route.tour.activities_slice(start_idx, end_idx).iter().fold((total, total), |(current, max), a| {
                    CapacityConstraintModule::<Capacity>::store_max_past_current_state(state, a, current, max)
                });

            acc.push(current);

            acc
        });

        starts.into_iter().zip(ends.into_iter()).for_each(|((start_idx, end_idx, _), end)| {
            route
                .tour
                .activities_slice(start_idx, end_idx)
                .iter()
                .rev()
                .fold(end, |max, a| CapacityConstraintModule::<Capacity>::store_max_future_state(state, a, max));
        });

        ctx.state_mut().put_route_state(MULTI_TOUR_KEY, last_multi_tour_index);
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for MultiTourCapacityConstraintModule<Capacity>
{
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        if get_tour_state(ctx).is_some() {
            Self::recalculate_states(ctx);
        } else {
            self.capacity_inner.accept_route_state(ctx);
            if Self::is_vehicle_full(ctx, &self.minimum) {
                ctx.state_mut().put_route_state(MULTI_TOUR_KEY, 0_usize);
            }
        }
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional_inner.accept_solution_state(ctx);
        // NOTE assume that multi tour jobs are locked in reader
        remove_trivial_tours(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

/// Locks multi tour jobs to specific vehicles
struct MultiTourHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for MultiTourHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        match job.as_ref() {
            Job::Single(job) => {
                if is_multi_tour_job(job) {
                    let vehicle_id = get_vehicle_id_from_job(job).unwrap();
                    let shift_index = get_shift_index(&job.dimens);

                    if !is_correct_vehicle(ctx, vehicle_id, shift_index) {
                        return Some(RouteConstraintViolation { code: self.code });
                    }
                }
            }
            Job::Multi(_) => {}
        }

        None
    }
}

struct MultiTourHardActivityConstraint {
    code: i32,
}

impl HardActivityConstraint for MultiTourHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(job) = as_multi_tour_job(activity_ctx.target) {
            // NOTE insert multi tour job in route only as last
            if activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some() {
                return Some(ActivityConstraintViolation { code: self.code, stopped: false });
            }
        }

        None
    }
}

/// Removes multi tours without jobs.
fn remove_trivial_tours(ctx: &mut SolutionContext) {
    // TODO remove multi tour job if next is final arrival or none (open vrp)
}

/// Checks whether tour index of job is applicable for current tour index of route.
fn is_time(ctx: &RouteContext, tour_index_job: usize) -> bool {
    if let Some(tour_index_state) = get_tour_state(ctx) {
        tour_index_state + 1 == tour_index_job
    } else {
        false
    }
}

fn is_multi_tour_job(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn as_multi_tour_job(activity: &Activity) -> Option<Arc<Single>> {
    as_single_job(activity, |job| is_multi_tour_job(job))
}

fn get_tour_state(ctx: &RouteContext) -> Option<usize> {
    ctx.state.get_route_state::<usize>(MULTI_TOUR_KEY).cloned()
}

fn get_tour_index(job: &Arc<Single>) -> Option<usize> {
    job.dimens.get_value::<usize>("tour_index").cloned()
}
