#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/capacity_test.rs"]
mod capacity_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, RouteState, SolutionContext};
use crate::models::common::{Dimensions, ValueDimension};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route, TourActivity};
use hashbrown::HashSet;
use std::iter::empty;
use std::marker::PhantomData;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

// TODO to avoid code duplication in generic type definition and implementation,

const CAPACITY_DIMENSION_KEY: &str = "cpc";
const DEMAND_DIMENSION_KEY: &str = "dmd";

/// Represents job demand, both static and dynamic.
pub struct Demand<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    /// Keeps static and dynamic pickup amount.
    pub pickup: (Capacity, Capacity),
    /// Keeps static and dynamic delivery amount.
    pub delivery: (Capacity, Capacity),
}

/// A trait to get or set capacity.
pub trait CapacityDimension<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    fn set_capacity(&mut self, demand: Capacity) -> &mut Self;
    fn get_capacity(&self) -> Option<&Capacity>;
}

/// A trait to get or set demand.
pub trait DemandDimension<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    fn set_demand(&mut self, demand: Demand<Capacity>) -> &mut Self;
    fn get_demand(&self) -> Option<&Demand<Capacity>>;
}

/// Returns intervals between vehicle terminal and reload activities.
pub fn route_intervals(route: &Route, is_reload: Box<dyn Fn(&TourActivity) -> bool + 'static>) -> Vec<(usize, usize)> {
    let last_idx = route.tour.total() - 1;
    (0_usize..).zip(route.tour.all_activities()).fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, a)| {
        if is_reload.deref()(a) || idx == last_idx {
            let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
            let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

            acc.push((start_idx, end_idx));
        }

        acc
    })
}

/// This trait defines multi-trip strategy.
pub trait MultiTrip<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    /// Returns true if job is reload.
    fn is_reload_job(&self, job: &Job) -> bool;

    /// Returns true if single job is reload.
    fn is_reload_single(&self, single: &Single) -> bool;

    /// Returns true if given job is reload and can be used with given route.
    fn is_assignable(&self, route: &Route, job: &Job) -> bool;

    /// Returns true when `current` capacity is close `max_capacity`.
    fn is_reload_needed(&self, current: &Capacity, max_capacity: &Capacity) -> bool;

    /// Returns true if route context has reloads.
    fn has_reloads(&self, route_ctx: &RouteContext) -> bool;

    /// Returns reload job from activity or None.
    fn get_reload<'a>(&self, activity: &'a Activity) -> Option<&'a Arc<Single>>;

    /// Gets all reloads for specific route from jobs collection.
    fn get_reloads<'a>(
        &'a self,
        route: &'a Route,
        jobs: &'a Vec<Job>,
    ) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync>;
}

/// A module which checks whether vehicle can handle customer's demand.
pub struct CapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    state_keys: Vec<i32>,
    conditional: ConditionalJobModule,
    constraints: Vec<ConstraintVariant>,
    multi_trip: Arc<dyn MultiTrip<Capacity> + Send + Sync>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    CapacityConstraintModule<Capacity>
{
    pub fn new(code: i32) -> Self {
        Self::new_with_multi_trip(code, Arc::new(NoMultiTrip { phantom: PhantomData }))
    }

    pub fn new_with_multi_trip(code: i32, multi_trip: Arc<dyn MultiTrip<Capacity> + Send + Sync>) -> Self {
        Self {
            state_keys: vec![CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY],
            conditional: ConditionalJobModule::new(Box::new(ConcreteJobContextTransition {
                remove_required: {
                    let multi_trip = multi_trip.clone();
                    move |_, job| multi_trip.is_reload_job(job)
                },
                promote_required: |_, _| false,
                remove_locked: |_, _| false,
                promote_locked: {
                    let multi_trip = multi_trip.clone();
                    move |_, job| multi_trip.is_reload_job(job)
                },
            })),
            constraints: vec![
                ConstraintVariant::SoftRoute(Arc::new(CapacitySoftRouteConstraint { multi_trip: multi_trip.clone() })),
                ConstraintVariant::HardRoute(Arc::new(CapacityHardRouteConstraint::<Capacity> {
                    code,
                    multi_trip: multi_trip.clone(),
                })),
                ConstraintVariant::HardActivity(Arc::new(CapacityHardActivityConstraint::<Capacity> {
                    code,
                    multi_trip: multi_trip.clone(),
                })),
            ],
            multi_trip,
        }
    }

    fn recalculate_states(&self, ctx: &mut RouteContext) {
        self.actualize_intervals(ctx).into_iter().fold(Capacity::default(), |acc, (start_idx, end_idx)| {
            let (route, state) = ctx.as_mut();

            // determine static deliveries loaded at the begin and static pickups brought to the end
            let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                (acc, Capacity::default()),
                |acc, activity| {
                    Self::get_demand(activity)
                        .and_then(|demand| Some((acc.0 + demand.delivery.0, acc.1 + demand.pickup.0)))
                        .unwrap_or_else(|| acc)
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

    fn actualize_intervals(&self, route_ctx: &mut RouteContext) -> Vec<(usize, usize)> {
        let (route, state) = route_ctx.as_mut();
        let intervals = route_intervals(route, {
            let multi_trip = self.multi_trip.clone();
            Box::new(move |a| multi_trip.get_reload(a).is_some())
        });
        state.put_route_state(RELOAD_INTERVALS, intervals.clone());

        intervals
    }

    fn is_vehicle_full(&self, ctx: &RouteContext) -> bool {
        ctx.route
            .tour
            .end()
            .map(|end| {
                self.multi_trip.is_reload_needed(
                    &ctx.state
                        .get_activity_state(MAX_PAST_CAPACITY_KEY, end)
                        .cloned()
                        .unwrap_or_else(|| Capacity::default()),
                    ctx.route.actor.vehicle.dimens.get_capacity().unwrap(),
                )
            })
            .unwrap_or(false)
    }

    /// Removes reloads at the start and end of tour.
    fn remove_trivial_reloads(&self, ctx: &mut SolutionContext) {
        if ctx.required.is_empty() {
            let mut extra_ignored = Vec::new();
            ctx.routes.iter_mut().filter(|ctx| self.multi_trip.has_reloads(ctx)).for_each(|rc| {
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
                    .filter_map(|(idx, activity)| self.multi_trip.get_reload(activity).map(|_| idx))
                    .filter(|&idx| idx < start || idx > end)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .for_each(|idx| {
                        let job = self.multi_trip.get_reload(rc.route.tour.get(idx).unwrap()).unwrap();
                        extra_ignored.push(Job::Single(job.clone()));
                        rc.route_mut().tour.remove_activity_at(idx);
                    });

                self.actualize_intervals(rc);
            });
            ctx.ignored.extend(extra_ignored.into_iter());
        }
    }

    fn has_demand_violation(
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

    fn can_handle_demand_on_intervals(
        ctx: &RouteContext,
        demand: Option<&Demand<Capacity>>,
        insert_idx: Option<usize>,
    ) -> bool {
        let has_demand_violation = |activity: &TourActivity| {
            CapacityConstraintModule::<Capacity>::has_demand_violation(
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
                if let Some(insert_idx) = insert_idx {
                    intervals.iter().filter(|(_, end_idx)| insert_idx <= *end_idx).all(|interval| {
                        has_demand_violation(ctx.route.tour.get(insert_idx.max(interval.0)).unwrap()).is_none()
                    })
                } else {
                    intervals
                        .iter()
                        .any(|(start_idx, _)| has_demand_violation(ctx.route.tour.get(*start_idx).unwrap()).is_none())
                }
            })
            .unwrap_or_else(|| has_demand_violation(ctx.route.tour.get(insert_idx.unwrap_or(0)).unwrap()).is_none())
    }

    fn get_demand(activity: &TourActivity) -> Option<&Demand<Capacity>> {
        activity.job.as_ref().and_then(|job| job.dimens.get_demand())
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for CapacityConstraintModule<Capacity>
{
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Job) {
        if self.multi_trip.is_reload_job(job) {
            // move all unassigned reloads back to ignored
            let jobs = self.multi_trip.get_reloads(&route_ctx.route, &solution_ctx.required).collect::<HashSet<_>>();
            solution_ctx.required.retain(|job| !jobs.contains(job));
            solution_ctx.ignored.extend(jobs.into_iter());

            self.accept_route_state(route_ctx);
        } else {
            self.accept_route_state(route_ctx);
            if self.is_vehicle_full(route_ctx) {
                // move all reloads for this shift to required
                let jobs = self
                    .multi_trip
                    .get_reloads(&route_ctx.route, &solution_ctx.ignored)
                    .chain(self.multi_trip.get_reloads(&route_ctx.route, &solution_ctx.required))
                    .collect::<HashSet<_>>();

                solution_ctx.ignored.retain(|job| !jobs.contains(job));
                solution_ctx.locked.extend(jobs.iter().cloned());
                solution_ctx.required.extend(jobs.into_iter());
            }
        }

        if solution_ctx.required.is_empty() {
            self.accept_solution_state(solution_ctx);
        }
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.recalculate_states(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.conditional.accept_solution_state(ctx);
        self.remove_trivial_reloads(ctx);

        ctx.routes.iter_mut().for_each(|route_ctx| {
            self.recalculate_states(route_ctx);
        })
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct CapacitySoftRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    multi_trip: Arc<dyn MultiTrip<Capacity> + Send + Sync>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    SoftRouteConstraint for CapacitySoftRouteConstraint<Capacity>
{
    fn estimate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> f64 {
        if self.multi_trip.is_reload_job(job) {
            0. - ctx.route.actor.vehicle.costs.fixed.max(1000.)
        } else {
            0.
        }
    }
}

/// Locks reload jobs to specific vehicles
struct CapacityHardRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    multi_trip: Arc<dyn MultiTrip<Capacity> + Send + Sync>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardRouteConstraint for CapacityHardRouteConstraint<Capacity>
{
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if self.multi_trip.is_reload_job(job) {
            return if self.multi_trip.is_assignable(&ctx.route, job) {
                None
            } else {
                Some(RouteConstraintViolation { code: self.code })
            };
        };

        let can_handle = match job {
            Job::Single(job) => {
                CapacityConstraintModule::<Capacity>::can_handle_demand_on_intervals(ctx, job.dimens.get_demand(), None)
            }
            Job::Multi(job) => job.jobs.iter().any(|job| {
                CapacityConstraintModule::<Capacity>::can_handle_demand_on_intervals(ctx, job.dimens.get_demand(), None)
            }),
        };

        if can_handle {
            None
        } else {
            Some(RouteConstraintViolation { code: self.code })
        }
    }
}

struct CapacityHardActivityConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    multi_trip: Arc<dyn MultiTrip<Capacity> + Send + Sync>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardActivityConstraint for CapacityHardActivityConstraint<Capacity>
{
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(_) = self.multi_trip.get_reload(activity_ctx.target) {
            // NOTE insert reload job in route only as last
            let is_first = activity_ctx.prev.job.is_none();
            let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

            return if is_first || is_not_last {
                Some(ActivityConstraintViolation { code: self.code, stopped: false })
            } else {
                None
            };
        };

        let demand = CapacityConstraintModule::<Capacity>::get_demand(activity_ctx.target);

        let violation = if activity_ctx.target.retrieve_job().map_or(false, |job| job.as_multi().is_some()) {
            // NOTE multi job has dynamic demand which can go in another interval
            if CapacityConstraintModule::<Capacity>::can_handle_demand_on_intervals(
                route_ctx,
                demand,
                Some(activity_ctx.index),
            ) {
                None
            } else {
                Some(false)
            }
        } else {
            CapacityConstraintModule::<Capacity>::has_demand_violation(
                &route_ctx.state,
                activity_ctx.prev,
                route_ctx.route.actor.vehicle.dimens.get_capacity(),
                demand,
                !self.multi_trip.has_reloads(route_ctx),
            )
        };

        if let Some(stopped) = violation {
            Some(ActivityConstraintViolation { code: self.code, stopped })
        } else {
            None
        }
    }
}

/// A no multi trip strategy.
struct NoMultiTrip<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    MultiTrip<Capacity> for NoMultiTrip<Capacity>
{
    fn is_reload_job(&self, _: &Job) -> bool {
        false
    }

    fn is_reload_single(&self, _: &Single) -> bool {
        false
    }

    fn is_assignable(&self, _: &Route, _: &Job) -> bool {
        false
    }

    fn is_reload_needed(&self, _: &Capacity, _: &Capacity) -> bool {
        false
    }

    fn has_reloads(&self, _: &RouteContext) -> bool {
        false
    }

    fn get_reload<'a>(&self, _: &'a Activity) -> Option<&'a Arc<Single>> {
        None
    }

    fn get_reloads<'a>(&'a self, _: &'a Route, _: &'a Vec<Job>) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync> {
        Box::new(empty())
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    Demand<Capacity>
{
    /// Returns capacity change as difference between pickup and delivery.
    pub fn change(&self) -> Capacity {
        self.pickup.0 + self.pickup.1 - self.delivery.0 - self.delivery.1
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Default
    for Demand<Capacity>
{
    fn default() -> Self {
        Self { pickup: (Default::default(), Default::default()), delivery: (Default::default(), Default::default()) }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Clone
    for Demand<Capacity>
{
    fn clone(&self) -> Self {
        Self { pickup: self.pickup, delivery: self.delivery }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    CapacityDimension<Capacity> for Dimensions
{
    fn set_capacity(&mut self, demand: Capacity) -> &mut Self {
        self.set_value(CAPACITY_DIMENSION_KEY, demand);
        self
    }

    fn get_capacity(&self) -> Option<&Capacity> {
        self.get_value(CAPACITY_DIMENSION_KEY)
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    DemandDimension<Capacity> for Dimensions
{
    fn set_demand(&mut self, demand: Demand<Capacity>) -> &mut Self {
        self.set_value(DEMAND_DIMENSION_KEY, demand);
        self
    }

    fn get_demand(&self) -> Option<&Demand<Capacity>> {
        self.get_value(DEMAND_DIMENSION_KEY)
    }
}
