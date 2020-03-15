use std::cmp::Ordering::Less;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{RouteContext, SolutionContext};
use vrp_core::models::common::Cost;
use vrp_core::models::problem::Job;

/// A module which provides way to balance work across all tours.
pub struct WorkBalanceModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl WorkBalanceModule {
    /// Creates `WorkBalanceModule` which balances max load across all tours.
    pub fn new_load_balanced<Capacity>(
        extra_cost: Cost,
        load_func: Box<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    ) -> Self
    where
        Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static,
    {
        Self {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(LoadBalanceSoftRouteConstraint::<Capacity> {
                load_func,
                extra_cost,
                default_capacity: Capacity::default(),
                default_intervals: vec![(0_usize, 0_usize)],
            }))],
            keys: vec![],
        }
    }
}

impl ConstraintModule for WorkBalanceModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct LoadBalanceSoftRouteConstraint<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    load_func: Box<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    extra_cost: Cost,
    default_capacity: Capacity,
    default_intervals: Vec<(usize, usize)>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    SoftRouteConstraint for LoadBalanceSoftRouteConstraint<Capacity>
{
    fn estimate_job(&self, ctx: &RouteContext, _job: &Job) -> f64 {
        let capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();

        let intervals =
            ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS).unwrap_or(&self.default_intervals);

        let max_load_ratio = intervals
            .iter()
            .map(|(start, _)| ctx.route.tour.get(*start).unwrap())
            .map(|activity| {
                ctx.state
                    .get_activity_state::<Capacity>(MAX_FUTURE_CAPACITY_KEY, activity)
                    .unwrap_or_else(|| &self.default_capacity)
            })
            .map(|max_load| self.load_func.deref()(max_load, capacity))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
            .unwrap_or(0_f64);

        (self.extra_cost + ctx.route.actor.vehicle.costs.fixed) * max_load_ratio
    }
}
