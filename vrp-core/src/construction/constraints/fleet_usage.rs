use crate::construction::constraints::{ConstraintModule, ConstraintVariant, SoftRouteConstraint};
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::Job;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// A module which controls fleet size usage.
pub struct FleetUsageConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for FleetUsageConstraintModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        Ok(source)
    }

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
        Self::new_with_cost(Box::new(|_| 1E12))
    }

    /// Creates `FleetUsageConstraintModule` to maximize used fleet size.
    pub fn new_maximized() -> Self {
        Self::new_with_cost(Box::new(|_| -1E12))
    }

    /// Creates `FleetUsageConstraintModule` to minimize total arrival time.
    pub fn new_earliest() -> Self {
        Self::new_with_cost(Box::new(|route_ctx| {
            // TODO find better approach to penalize later departures
            route_ctx.route.actor.detail.time.start
        }))
    }

    fn new_with_cost(extra_cost_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>) -> Self {
        Self {
            state_keys: vec![],
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(FleetCostSoftRouteConstraint { extra_cost_fn }))],
        }
    }
}

struct FleetCostSoftRouteConstraint {
    extra_cost_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>,
}

impl SoftRouteConstraint for FleetCostSoftRouteConstraint {
    fn estimate_job(&self, _: &SolutionContext, route_ctx: &RouteContext, _job: &Job) -> Cost {
        if route_ctx.route.tour.job_count() == 0 {
            self.extra_cost_fn.deref()(route_ctx)
        } else {
            0.
        }
    }
}
