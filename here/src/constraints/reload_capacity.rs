use crate::constraints::{as_single_job, get_shift_index, get_vehicle_id_from_job, is_correct_vehicle};
use core::construction::constraints::*;
use core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use core::models::common::ValueDimension;
use core::models::problem::{Job, Single};
use core::models::solution::Activity;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

const RELOAD_INDEX_KEY: i32 = 101;

pub struct ReloadCapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>,
    state_keys: Vec<i32>,
    capacity_inner: CapacityConstraintModule<Capacity>,
    conditional_inner: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ReloadCapacityConstraintModule<Capacity>
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
            state_keys: capacity_constraint.state_keys().chain(vec![RELOAD_INDEX_KEY].iter()).cloned().collect(),
            capacity_inner: capacity_constraint,
            conditional_inner: ConditionalJobModule::new(
                Some(Box::new(move |_, job| !is_reload_job(job))),
                Some(Box::new(move |_, job| is_reload_job(job))),
            ),
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(ReloadHardRouteConstraint { code, hard_route_constraint })),
                ConstraintVariant::HardActivity(Arc::new(ReloadHardActivityConstraint::<Capacity> {
                    code,
                    hard_activity_constraint,
                    phantom: PhantomData,
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

        let last_idx = route.tour.total() - 1;
        let (_, _, starts) = (0_usize..).zip(route.tour.all_activities()).fold(
            (Capacity::default(), Capacity::default(), Vec::<(usize, usize, Capacity)>::default()),
            |(start_total, end_total, mut acc), (idx, a)| {
                let demand = Demand::<Capacity>::default();
                let demand = CapacityConstraintModule::<Capacity>::get_demand(a).unwrap_or(&demand);
                let (start_total, end_total) = if as_reload_job(a).is_some() || idx == last_idx {
                    let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
                    let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

                    acc.push((start_idx, end_idx, start_total));

                    (end_total, Capacity::default())
                } else {
                    (start_total + demand.delivery.0, end_total + demand.pickup.1 - demand.delivery.1)
                };

                (start_total, end_total, acc)
            },
        );

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
    ConstraintModule for ReloadCapacityConstraintModule<Capacity>
{
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Arc<Job>) {
        if is_reload_job(job) {
            let reload_idx = get_reload_index_from_job(&job.as_single()).unwrap();
            route_ctx.state_mut().put_route_state(RELOAD_INDEX_KEY, reload_idx);
            self.accept_route_state(route_ctx);
        } else {
            self.accept_route_state(route_ctx);
            if Self::is_vehicle_full(route_ctx, &self.threshold) {
                let next_reload_idx = get_reload_index_from_route(route_ctx).unwrap_or(0) + 1;
                let shift_index = get_shift_index(&route_ctx.route.actor.vehicle.dimens);

                let index = solution_ctx.ignored.iter().position(move |job| match job.as_ref() {
                    Job::Single(job) => {
                        is_reload_single(&job)
                            && get_shift_index(&job.dimens) == shift_index
                            && get_reload_index_from_job(&job).unwrap() == next_reload_idx
                    }
                    _ => false,
                });

                if let Some(index) = index {
                    let job = solution_ctx.ignored.remove(index);
                    solution_ctx.required.push(job.clone());
                    solution_ctx.locked.insert(job);
                }
            }
        }

        remove_trivial_reloads(solution_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        if has_reload_index(ctx) {
            Self::recalculate_states(ctx);
        } else {
            self.capacity_inner.accept_route_state(ctx);
        }
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // NOTE promote reload jobs to ignored and locked
        if ctx.routes.iter().find(|rc| has_reload_index(rc)).is_none() {
            self.conditional_inner.accept_solution_state(ctx);
        }

        remove_trivial_reloads(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

/// Locks reload jobs to specific vehicles
struct ReloadHardRouteConstraint {
    code: i32,
    hard_route_constraint: Arc<dyn HardRouteConstraint + Send + Sync>,
}

impl HardRouteConstraint for ReloadHardRouteConstraint {
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

        if has_reload_index(ctx) {
            // TODO can we do some checks here?
            None
        } else {
            self.hard_route_constraint.evaluate_job(ctx, job)
        }
    }
}

struct ReloadHardActivityConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    hard_activity_constraint: Arc<dyn HardActivityConstraint + Send + Sync>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardActivityConstraint for ReloadHardActivityConstraint<Capacity>
{
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

        if has_reload_index(route_ctx) {
            let multi = activity_ctx.target.retrieve_job().and_then(|job| match job.as_ref() {
                Job::Multi(multi) => Some((job.clone(), multi.jobs.len())),
                _ => None,
            });

            if let Some((job, singles)) = multi {
                let processed_activities = route_ctx.route.tour.job_activities(&job).count();
                // NOTE check capacity violation for reloads
                if processed_activities == singles - 1 {
                    let capacity: Capacity = *route_ctx.route.actor.vehicle.dimens.get_capacity().unwrap();
                    let index = route_ctx.route.tour.activity_index(activity_ctx.prev).unwrap();

                    // TODO optimize this?
                    let has_violation = route_ctx.route.tour.activities_slice(0, index).iter().rev().any(|a| {
                        *route_ctx.state.get_activity_state::<Capacity>(MAX_PAST_CAPACITY_KEY, a).unwrap() > capacity
                    });

                    if has_violation {
                        return Some(ActivityConstraintViolation { code: self.code, stopped: false });
                    }
                }
            }
        }

        self.hard_activity_constraint.evaluate_activity(route_ctx, activity_ctx)
    }
}

/// Removes reloads at the end of tour.
fn remove_trivial_reloads(ctx: &mut SolutionContext) {
    if ctx.required.is_empty() {
        ctx.routes.iter_mut().for_each(|rc| {
            let activities = rc.route.tour.total();
            let last_reload_idx = if rc.route.actor.detail.end.is_some() { activities - 2 } else { activities - 1 };

            if as_reload_job(rc.route.tour.get(last_reload_idx).unwrap()).is_some() {
                rc.route_mut().tour.remove_activity_at(last_reload_idx);
            }
        });
    }
}

fn is_reload_single(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn is_reload_job(job: &Arc<Job>) -> bool {
    match job.as_ref() {
        Job::Single(job) => is_reload_single(job),
        _ => false,
    }
}

fn as_reload_job(activity: &Activity) -> Option<Arc<Single>> {
    as_single_job(activity, |job| is_reload_single(job))
}

fn has_reload_index(ctx: &RouteContext) -> bool {
    get_reload_index_from_route(ctx).is_some()
}

fn get_reload_index_from_route(ctx: &RouteContext) -> Option<usize> {
    ctx.state.get_route_state::<usize>(RELOAD_INDEX_KEY).cloned()
}

fn get_reload_index_from_job(job: &Arc<Single>) -> Option<usize> {
    job.dimens.get_value::<usize>("reload_index").cloned()
}
