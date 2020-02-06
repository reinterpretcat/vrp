#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/capacity_test.rs"]
mod capacity_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, RouteState, SolutionContext};
use crate::models::common::{Dimensions, ValueDimension};
use crate::models::problem::Job;
use crate::models::solution::TourActivity;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::slice::Iter;
use std::sync::Arc;

// TODO to avoid code duplication in generic type definition and implementation,
// TODO consider to use TODO trait aliases once they are stabilized (or macro?).

/// Represents job demand, both static and dynamic.
pub struct Demand<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    /// Keeps static and dynamic pickup amount.
    pub pickup: (Capacity, Capacity),
    /// Keeps static and dynamic delivery amount.
    pub delivery: (Capacity, Capacity),
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    Demand<Capacity>
{
    /// Returns capacity change as difference between pickup and delivery.
    fn change(&self) -> Capacity {
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

/// A module which checks whether vehicle can handle customer's demand.
pub struct CapacityConstraintModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    CapacityConstraintModule<Capacity>
{
    /// Creates a new [`CapacityConstraintModule`].
    pub fn new(code: i32) -> Self {
        Self {
            state_keys: vec![CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(CapacityHardRouteConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
                ConstraintVariant::HardActivity(Arc::new(CapacityHardActivityConstraint::<Capacity> {
                    code,
                    phantom: PhantomData,
                })),
            ],
            phantom: PhantomData,
        }
    }

    /// A helper method to return demand defined on tour activity.
    pub fn get_demand(activity: &TourActivity) -> Option<&Demand<Capacity>> {
        activity.job.as_ref().and_then(|job| job.dimens.get_demand())
    }

    /// Checks whether demand can be handled.
    pub fn can_handle_demand(
        state: &RouteState,
        pivot: &TourActivity,
        capacity: Option<&Capacity>,
        demand: Option<&Demand<Capacity>>,
        code: i32,
        can_stop: bool,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(demand) = demand {
            if let Some(&capacity) = capacity {
                let default = Capacity::default();

                // cannot handle more static deliveries
                if demand.delivery.0 > default {
                    let past = *state.get_activity_state(MAX_PAST_CAPACITY_KEY, pivot).unwrap_or(&default);
                    if past + demand.delivery.0 > capacity {
                        return Some(ActivityConstraintViolation { code, stopped: can_stop });
                    }
                }

                let change = demand.change();

                // cannot handle more pickups
                if change > default {
                    let future = *state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot).unwrap_or(&default);
                    if future + change > capacity {
                        return Some(ActivityConstraintViolation { code, stopped: can_stop });
                    }
                }

                // can load more at current
                let current = *state.get_activity_state(CURRENT_CAPACITY_KEY, pivot).unwrap_or(&default);

                if current + change <= capacity {
                    None
                } else {
                    Some(ActivityConstraintViolation { code, stopped: false })
                }
            } else {
                Some(ActivityConstraintViolation { code, stopped: can_stop })
            }
        } else {
            None
        }
    }

    /// Stores max past current state inside route state.
    pub fn store_max_past_current_state(
        state: &mut RouteState,
        activity: &TourActivity,
        current: Capacity,
        max: Capacity,
    ) -> (Capacity, Capacity) {
        let current = current + Self::get_demand(activity).unwrap_or(&Demand::<Capacity>::default()).change();
        let max = std::cmp::max(max, current);

        state.put_activity_state(CURRENT_CAPACITY_KEY, activity, current);
        state.put_activity_state(MAX_PAST_CAPACITY_KEY, activity, max);

        (current, max)
    }

    /// Stores max future current state inside route state.
    pub fn store_max_future_state(state: &mut RouteState, activity: &TourActivity, max: Capacity) -> Capacity {
        let max = std::cmp::max(max, *state.get_activity_state(CURRENT_CAPACITY_KEY, activity).unwrap());
        state.put_activity_state(MAX_FUTURE_CAPACITY_KEY, activity, max);
        max
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for CapacityConstraintModule<Capacity>
{
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, _job: &Job) {
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let (route, state) = ctx.as_mut();

        let start = route.tour.all_activities().fold(Capacity::default(), |total, a| {
            total + Self::get_demand(a).map_or(Capacity::default(), |d| d.delivery.0)
        });

        let (end, _) = route
            .tour
            .all_activities()
            .fold((start, start), |(current, max), a| Self::store_max_past_current_state(state, a, current, max));

        route.tour.all_activities().rev().fold(end, |max, a| Self::store_max_future_state(state, a, max));
    }

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

const CAPACITY_DIMENSION_KEY: &str = "cpc";
const DEMAND_DIMENSION_KEY: &str = "dmd";

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

struct CapacityHardRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardRouteConstraint for CapacityHardRouteConstraint<Capacity>
{
    fn evaluate_job(&self, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        match job {
            Job::Single(job) => {
                if CapacityConstraintModule::<Capacity>::can_handle_demand(
                    &ctx.state,
                    ctx.route.tour.start().unwrap_or_else(|| unimplemented!("Optional start is not yet implemented.")),
                    ctx.route.actor.vehicle.dimens.get_capacity(),
                    job.dimens.get_demand(),
                    self.code,
                    true,
                )
                .is_none()
                {
                    None
                } else {
                    Some(RouteConstraintViolation { code: self.code })
                }
            }
            // TODO we can check at least static pickups/deliveries
            _ => None,
        }
    }
}

struct CapacityHardActivityConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    code: i32,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    HardActivityConstraint for CapacityHardActivityConstraint<Capacity>
{
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let demand = CapacityConstraintModule::<Capacity>::get_demand(activity_ctx.target);

        let can_stop = activity_ctx
            .target
            .retrieve_job()
            .and_then(|job| job.as_single().map_or(None, |_| Some(true)))
            .unwrap_or(false);

        CapacityConstraintModule::<Capacity>::can_handle_demand(
            &route_ctx.state,
            activity_ctx.prev,
            route_ctx.route.actor.vehicle.dimens.get_capacity(),
            demand,
            self.code,
            can_stop,
        )
    }
}
