use crate::construction::constraints::{ConstraintModule, ConstraintVariant, SoftRouteConstraint};
use crate::construction::states::{RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::Job;
use std::slice::Iter;
use std::sync::Arc;

/// A module which controls fleet size usage.
pub struct FleetUsageConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for FleetUsageConstraintModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl FleetUsageConstraintModule {
    /// Creates `FleetUsageConstraintModule` to minimize used fleet size.
    pub fn new_minimized() -> Self {
        Self::new_with_cost(1E12)
    }

    /// Creates `FleetUsageConstraintModule` to maximize used fleet size.
    pub fn new_maximized() -> Self {
        Self::new_with_cost(-1E12)
    }

    /// Creates `FleetUsageConstraintModule` with custom extra cost.
    pub fn new_with_cost(extra_cost: Cost) -> Self {
        Self {
            state_keys: vec![],
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(FleetCostSoftRouteConstraint { extra_cost }))],
        }
    }
}

struct FleetCostSoftRouteConstraint {
    extra_cost: Cost,
}

impl SoftRouteConstraint for FleetCostSoftRouteConstraint {
    fn estimate_job(&self, ctx: &RouteContext, _job: &Job) -> Cost {
        if ctx.route.tour.job_count() == 0 {
            self.extra_cost
        } else {
            0.
        }
    }
}
