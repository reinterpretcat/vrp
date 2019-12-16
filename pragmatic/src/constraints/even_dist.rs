use crate::constraints::MAX_TOUR_LOAD_KEY;
use core::construction::constraints::*;
use core::construction::states::{RouteContext, SolutionContext};
use core::models::common::Cost;
use core::models::problem::Job;
use std::marker::PhantomData;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

/**
    Adds some extra penalty to use loaded vehicle for job insertion.
    Can be used to control job distribution across the fleet.
*/
pub struct EvenDistributionModule<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    EvenDistributionModule<Capacity>
{
    pub fn new(extra_cost: Cost, load_func: Box<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>) -> Self {
        Self {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(
                EvenDistributionSoftRouteConstraint::<Capacity> {
                    load_func,
                    extra_cost,
                    default_capacity: Capacity::default(),
                },
            ))],
            keys: vec![],
            phantom: PhantomData,
        }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ConstraintModule for EvenDistributionModule<Capacity>
{
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Arc<Job>) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct EvenDistributionSoftRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    load_func: Box<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    extra_cost: Cost,
    default_capacity: Capacity,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    SoftRouteConstraint for EvenDistributionSoftRouteConstraint<Capacity>
{
    fn estimate_job(&self, ctx: &RouteContext, _job: &Arc<Job>) -> f64 {
        let capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();
        let max_load = ctx
            .state
            .get_route_state::<Capacity>(MAX_TOUR_LOAD_KEY)
            .or_else(|| {
                ctx.state.get_activity_state::<Capacity>(MAX_FUTURE_CAPACITY_KEY, ctx.route.tour.start().unwrap())
            })
            .unwrap_or_else(|| &self.default_capacity);

        let load_ratio = self.load_func.deref()(max_load, capacity);

        (self.extra_cost + ctx.route.actor.vehicle.costs.fixed) * load_ratio
    }
}
