use crate::constraints::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{ActivityContext, RouteContext, RouteState, SolutionContext};
use vrp_core::models::common::{Cost, IdDimension, ValueDimension};
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::{Activity, Route, TourActivity};

pub struct ReloadCapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>,
    state_keys: Vec<i32>,
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
}

/// Returns intervals between depots and reload points.
pub fn reload_intervals(route: &Route) -> Vec<(usize, usize)> {
    let last_idx = route.tour.total() - 1;
    (0_usize..).zip(route.tour.all_activities()).fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, a)| {
        if as_reload_job(a).is_some() || idx == last_idx {
            let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
            let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

            acc.push((start_idx, end_idx));
        }

        acc
    })
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ReloadCapacityConstraintModule<Capacity>
{
    pub fn new(code: i32, cost_reward: Cost, threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> Self {
        Self {
            threshold,
            state_keys: vec![CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY],
            conditional: ConditionalJobModule::new(create_job_transition()),
            constraints: vec![
                ConstraintVariant::SoftRoute(Arc::new(ReloadSoftRouteConstraint { cost: cost_reward })),
                ConstraintVariant::HardRoute(Arc::new(ReloadHardRouteConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
                ConstraintVariant::HardActivity(Arc::new(ReloadHardActivityConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
            ],
        }
    }

    fn is_vehicle_full(rc: &RouteContext, threshold: &Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> bool {
        let tour = &rc.route.tour;
        let state = &rc.state;

        if let Some(end) = tour.end() {
            let max_capacity = threshold(rc.route.actor.vehicle.dimens.get_capacity().unwrap());
            let load =
                state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).cloned().unwrap_or_else(|| Capacity::default());

            load >= max_capacity
        } else {
            false
        }
    }

    /// Checks whether demand can be handled at interval starts at given pivot activity.
    fn can_handle_demand(
        state: &RouteState,
        pivot: &TourActivity,
        capacity: Option<&Capacity>,
        demand: Option<&Demand<Capacity>>,
        stopped: bool,
    ) -> Option<bool> {
        if let Some(demand) = demand {
            if let Some(&capacity) = capacity {
                let default = Capacity::default();

                // cannot handle more static deliveries
                if demand.delivery.0 > default {
                    let past = *state.get_activity_state(MAX_PAST_CAPACITY_KEY, pivot).unwrap_or(&default);
                    if past + demand.delivery.0 > capacity {
                        return Some(stopped);
                    }
                }

                let change = demand.change();

                // cannot handle more pickups
                if change > default {
                    let future = *state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot).unwrap_or(&default);
                    if future + change > capacity {
                        return Some(stopped);
                    }
                }

                // can load more at current
                let current = *state.get_activity_state(CURRENT_CAPACITY_KEY, pivot).unwrap_or(&default);

                if current + change <= capacity {
                    None
                } else {
                    Some(false)
                }
            } else {
                Some(stopped)
            }
        } else {
            None
        }
    }

    fn can_handle_demand_on_intervals(ctx: &RouteContext, demand: Option<&Demand<Capacity>>) -> bool {
        let can_handle_demand = |activity: &TourActivity| {
            ReloadCapacityConstraintModule::<Capacity>::can_handle_demand(
                &ctx.state,
                activity,
                ctx.route.actor.vehicle.dimens.get_capacity(),
                demand,
                true,
            )
        };

        ctx.state
            .get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS)
            .map(|intervals| {
                intervals
                    .iter()
                    .any(|(start_idx, _)| can_handle_demand(ctx.route.tour.get(*start_idx).unwrap()).is_none())
            })
            .unwrap_or_else(|| {
                can_handle_demand(
                    ctx.route.tour.start().unwrap_or_else(|| unimplemented!("Optional start is not yet implemented.")),
                )
                .is_none()
            })
    }

    fn get_demand(activity: &TourActivity) -> Option<&Demand<Capacity>> {
        activity.job.as_ref().and_then(|job| job.dimens.get_demand())
    }

    /// Removes reloads at the start and end of tour.
    fn remove_trivial_reloads(ctx: &mut SolutionContext) {
        if ctx.required.is_empty() {
            let mut extra_ignored = Vec::new();
            ctx.routes.iter_mut().for_each(|rc| {
                let demands = (0..)
                    .zip(rc.route.tour.all_activities())
                    .filter_map(|(idx, activity)| Self::get_demand(activity).map(|_| idx))
                    .collect::<Vec<_>>();

                let (start, end) = (
                    demands.first().cloned().unwrap_or(0),
                    demands.last().cloned().unwrap_or(rc.route.tour.total() - 1),
                );

                (0..)
                    .zip(rc.route.tour.all_activities())
                    .filter_map(|(idx, activity)| as_reload_job(activity).map(|_| idx))
                    .filter(|&idx| idx < start || idx > end)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .for_each(|idx| {
                        let job = as_reload_job(rc.route.tour.get(idx).unwrap()).unwrap();
                        extra_ignored.push(Job::Single(job.clone()));
                        rc.route_mut().tour.remove_activity_at(idx);
                    });
            });
            ctx.ignored.extend(extra_ignored.into_iter());
        }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for ReloadCapacityConstraintModule<Capacity>
{
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Job) {
        if is_reload_job(job) {
            // move all unassigned reloads back to ignored
            let jobs = get_reload_jobs(route_ctx, &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs.into_iter());

            self.accept_route_state(route_ctx);
        } else {
            self.accept_route_state(route_ctx);
            if Self::is_vehicle_full(route_ctx, &self.threshold) {
                // move all reloads for this shift to required
                let jobs = get_reload_jobs(route_ctx, &solution_ctx.ignored)
                    .chain(get_reload_jobs(route_ctx, &solution_ctx.required))
                    .collect::<HashSet<_>>();

                solution_ctx.ignored.retain(|job| !jobs.contains(job));
                solution_ctx.locked.extend(jobs.iter().cloned());
                solution_ctx.required.extend(jobs.into_iter());
            }
        }

        Self::remove_trivial_reloads(solution_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let (route, state) = ctx.as_mut();

        let intervals = reload_intervals(route);
        state.put_route_state(RELOAD_INTERVALS, intervals.clone());

        intervals.into_iter().fold(Capacity::default(), |acc, (start_idx, end_idx)| {
            // determine static deliveries loaded at the begin and static pickups brought to the end
            let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                (acc, Capacity::default()),
                |acc, activity| {
                    let (delivery, pickup) = Self::get_demand(activity)
                        .and_then(|demand| Some((demand.delivery.0, demand.pickup.0)))
                        .unwrap_or_else(|| (Capacity::default(), Capacity::default()));

                    (acc.0 + delivery, acc.1 + pickup)
                },
            );

            // determine actual load at each activity and max discovered in the past
            let (current, _) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                (start_delivery, Capacity::default()),
                |(current, max), activity| {
                    let change =
                        Self::get_demand(activity).map(|demand| demand.change()).unwrap_or_else(|| Capacity::default());

                    let current = current + change;
                    let max = std::cmp::max(max, current);

                    state.put_activity_state(CURRENT_CAPACITY_KEY, activity, current);
                    state.put_activity_state(MAX_PAST_CAPACITY_KEY, activity, max);

                    (current, max)
                },
            );

            route.tour.activities_slice(start_idx, end_idx).iter().rev().fold(current, |max, activity| {
                let max = std::cmp::max(max, *state.get_activity_state(CURRENT_CAPACITY_KEY, activity).unwrap());
                state.put_activity_state(MAX_FUTURE_CAPACITY_KEY, activity, max);
                max
            });

            current - end_pickup
        });
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
        Self::remove_trivial_reloads(ctx);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct ReloadSoftRouteConstraint {
    cost: Cost,
}

impl SoftRouteConstraint for ReloadSoftRouteConstraint {
    fn estimate_job(&self, ctx: &RouteContext, job: &Job) -> f64 {
        if is_reload_job(job) {
            -ctx.route.actor.vehicle.costs.fixed - self.cost
        } else {
            0.
        }
    }
}

/// Locks reload jobs to specific vehicles
struct ReloadHardRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardRouteConstraint for ReloadHardRouteConstraint<Capacity>
{
    fn evaluate_job(&self, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if is_reload_job(job) {
            let job = job.to_single();
            let vehicle_id = get_vehicle_id_from_job(&job).unwrap();
            let shift_index = get_shift_index(&job.dimens);

            return if !is_correct_vehicle(ctx, vehicle_id, shift_index) {
                Some(RouteConstraintViolation { code: self.code })
            } else {
                None
            };
        };

        let can_handle = match job {
            Job::Single(job) => {
                ReloadCapacityConstraintModule::<Capacity>::can_handle_demand_on_intervals(ctx, job.dimens.get_demand())
            }
            Job::Multi(job) => job.jobs.iter().any(|job| {
                ReloadCapacityConstraintModule::<Capacity>::can_handle_demand_on_intervals(ctx, job.dimens.get_demand())
            }),
        };

        if can_handle {
            None
        } else {
            Some(RouteConstraintViolation { code: self.code })
        }
    }
}

struct ReloadHardActivityConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
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
            let is_first = activity_ctx.prev.job.is_none();
            let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

            return if is_first || is_not_last {
                Some(ActivityConstraintViolation { code: self.code, stopped: false })
            } else {
                None
            };
        };

        if let Some(stopped) = ReloadCapacityConstraintModule::<Capacity>::can_handle_demand(
            &route_ctx.state,
            activity_ctx.prev,
            route_ctx.route.actor.vehicle.dimens.get_capacity(),
            CapacityConstraintModule::<Capacity>::get_demand(activity_ctx.target),
            !has_reloads(route_ctx),
        ) {
            Some(ActivityConstraintViolation { code: self.code, stopped })
        } else {
            None
        }
    }
}

/// Creates job transition which removes reload jobs from required and adds them to locked.
fn create_job_transition() -> Box<dyn JobContextTransition + Send + Sync> {
    Box::new(ConcreteJobContextTransition {
        remove_required: |_, job| is_reload_job(job),
        promote_required: |_, _| false,
        remove_locked: |_, _| false,
        promote_locked: |_, job| is_reload_job(job),
    })
}

fn is_reload_single(job: &Arc<Single>) -> bool {
    job.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn is_reload_job(job: &Job) -> bool {
    job.as_single().map_or(false, |single| is_reload_single(single))
}

fn as_reload_job(activity: &Activity) -> Option<&Arc<Single>> {
    as_single_job(activity, |job| is_reload_single(job))
}

fn has_reloads(ctx: &RouteContext) -> bool {
    ctx.state
        .get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS)
        .map(|intervals| intervals.len() > 1)
        .unwrap_or(false)
}

fn get_reload_jobs<'a>(route_ctx: &'a RouteContext, collection: &'a Vec<Job>) -> Box<dyn Iterator<Item = Job> + 'a> {
    let shift_index = get_shift_index(&route_ctx.route.actor.vehicle.dimens);
    let vehicle_id = route_ctx.route.actor.vehicle.dimens.get_id().unwrap();

    Box::new(
        collection
            .iter()
            .filter(move |job| match job {
                Job::Single(job) => {
                    is_reload_single(&job)
                        && get_shift_index(&job.dimens) == shift_index
                        && get_vehicle_id_from_job(&job).unwrap() == vehicle_id
                }
                _ => false,
            })
            .cloned(),
    )
}
