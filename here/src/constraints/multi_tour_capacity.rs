use crate::constraints::{as_single_job, get_shift_index, get_vehicle_id_from_job, is_correct_vehicle};
use core::construction::constraints::*;
use core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use core::models::common::ValueDimension;
use core::models::problem::{Job, Single};
use core::models::solution::Activity;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

const MULTI_TOUR_INDEX_KEY: i32 = 101;

pub struct MultiTourCapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>,
    state_keys: Vec<i32>,
    capacity_inner: CapacityConstraintModule<Capacity>,
    conditional_inner: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    MultiTourCapacityConstraintModule<Capacity>
{
    pub fn new(code: i32, threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> Self {
        let capacity_constraint = CapacityConstraintModule::<Capacity>::new(code);
        let hard_route_constraint = capacity_constraint
            .get_constraints()
            .filter_map(|c| match c {
                ConstraintVariant::HardRoute(c) => Some(c.clone()),
                _ => None,
            })
            .next()
            .unwrap();

        let hard_activity_constraint = capacity_constraint
            .get_constraints()
            .filter_map(|c| match c {
                ConstraintVariant::HardActivity(c) => Some(c.clone()),
                _ => None,
            })
            .next()
            .unwrap();

        Self {
            threshold,
            state_keys: capacity_constraint.state_keys().chain(vec![MULTI_TOUR_INDEX_KEY].iter()).cloned().collect(),
            capacity_inner: capacity_constraint,
            conditional_inner: ConditionalJobModule::new(
                Some(Box::new(move |_, job| !is_reload_job(job))),
                Some(Box::new(move |_, job| is_reload_job(job))),
            ),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(MultiTourHardRouteConstraint { code, hard_route_constraint })),
                ConstraintVariant::HardActivity(Arc::new(MultiTourHardActivityConstraint {
                    code,
                    hard_activity_constraint,
                })),
            ],
        }
    }

    fn is_vehicle_full(rc: &RouteContext, threshold: &Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> bool {
        let tour = &rc.route.tour;
        let state = &rc.state;

        if let Some(end) = tour.end() {
            let empty_capacity = Capacity::default();
            let max_capacity = threshold(rc.route.actor.vehicle.dimens.get_capacity().unwrap());

            let load = *state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).unwrap_or_else(|| &empty_capacity);

            load >= max_capacity
        } else {
            false
        }
    }

    fn recalculate_states(ctx: &mut RouteContext) {
        let (route, state) = ctx.as_mut();

        // TODO check what happens here

        let last_idx = route.tour.total() - 1;
        let (_, starts) = (0_usize..).zip(route.tour.all_activities()).fold(
            (Capacity::default(), Vec::<(usize, usize, Capacity)>::default()),
            |(total, mut acc), (idx, a)| {
                let total = if as_reload_job(a).is_some() || idx == last_idx {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
                    let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

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

        //let last_multi_tour_index = starts.len() - 1;

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
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for MultiTourCapacityConstraintModule<Capacity>
{
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        if get_reload_index(ctx).is_some() {
            Self::recalculate_states(ctx);
        } else {
            self.capacity_inner.accept_route_state(ctx);
        }

        if Self::is_vehicle_full(ctx, &self.threshold) {
            let next_reload_idx = get_reload_index(ctx).unwrap_or(0) + 1;
            ctx.state_mut().put_route_state(MULTI_TOUR_INDEX_KEY, next_reload_idx);
        }
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        let mut routes_with_reload =
            ctx.routes.iter().filter(|rc| get_reload_index(rc).is_some()).cloned().collect::<Vec<_>>();

        // NOTE promote reload jobs only when they are not yet used
        // TODO do it once
        if routes_with_reload.is_empty() {
            self.conditional_inner.accept_solution_state(ctx);
        }

        routes_with_reload.iter_mut().for_each(|rc| {
            let shift_index = get_shift_index(&rc.route.actor.vehicle.dimens);
            let reload_idx = get_reload_index(rc).unwrap();

            let index = ctx.ignored.iter().position(move |job| match job.as_ref() {
                Job::Single(job) => {
                    is_reload_single(&job)
                        && get_shift_index(&job.dimens) == shift_index
                        && get_tour_index(&job).unwrap() == reload_idx
                }
                _ => false,
            });

            if let Some(index) = index {
                ctx.required.push(ctx.ignored.remove(index));
                //rc.state_mut().put_route_state(MULTI_TOUR_FULL_KEY, false);
            }
        });

        if ctx.required.is_empty() {
            // NOTE assume that multi tour jobs are locked in reader
            remove_trivial_tours(ctx);
        }
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
    hard_route_constraint: Arc<dyn HardRouteConstraint + Send + Sync>,
}

impl HardRouteConstraint for MultiTourHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        if is_reload_job(job) {
            let job = job.as_single();
            let vehicle_id = get_vehicle_id_from_job(&job).unwrap();
            let shift_index = get_shift_index(&job.dimens);

            return if !is_correct_vehicle(ctx, vehicle_id, shift_index) {
                Some(RouteConstraintViolation { code: self.code })
            } else {
                None
            };
        }

        if get_reload_index(ctx).is_none() {
            self.hard_route_constraint.evaluate_job(ctx, job)
        } else {
            // TODO can we do some checks here?
            None
        }
    }
}

struct MultiTourHardActivityConstraint {
    code: i32,
    hard_activity_constraint: Arc<dyn HardActivityConstraint + Send + Sync>,
}

impl HardActivityConstraint for MultiTourHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(_) = as_reload_job(activity_ctx.target) {
            // NOTE insert reload job in route only as last
            return if activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some() {
                Some(ActivityConstraintViolation { code: self.code, stopped: false })
            } else {
                None
            };
        }

        self.hard_activity_constraint.evaluate_activity(route_ctx, activity_ctx)
    }
}

/// Removes multi tours without jobs.
fn remove_trivial_tours(_ctx: &mut SolutionContext) {
    // TODO remove multi tour job if next is final arrival or none (open vrp)
}

fn is_reload_single(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn is_reload_job(job: &Arc<Job>) -> bool {
    match job.as_ref() {
        Job::Single(job) => is_reload_single(job),
        _ => true,
    }
}

fn as_reload_job(activity: &Activity) -> Option<Arc<Single>> {
    as_single_job(activity, |job| is_reload_single(job))
}

fn get_reload_index(ctx: &RouteContext) -> Option<usize> {
    ctx.state.get_route_state::<usize>(MULTI_TOUR_INDEX_KEY).cloned()
}

fn get_tour_index(job: &Arc<Single>) -> Option<usize> {
    job.dimens.get_value::<usize>("tour_index").cloned()
}
