use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{RouteContext, SolutionContext};
use vrp_core::models::common::Cost;
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::Job;

/** Adds some extra penalty to jobs with priority bigger than 1. */
pub struct PriorityModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl PriorityModule {
    pub fn new(extra_cost: Cost) -> Self {
        Self {
            constraints: vec![ConstraintVariant::SoftRoute(Arc::new(PrioritySoftRouteConstraint { extra_cost }))],
            keys: vec![],
        }
    }
}

impl ConstraintModule for PriorityModule {
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

struct PrioritySoftRouteConstraint {
    extra_cost: Cost,
}

impl SoftRouteConstraint for PrioritySoftRouteConstraint {
    fn estimate_job(&self, _: &RouteContext, job: &Job) -> f64 {
        match job {
            Job::Single(job) => job.dimens.get_value::<i32>("priority"),
            Job::Multi(job) => job.dimens.get_value::<i32>("priority"),
        }
        .map_or(0., |priority| ((priority - 1) as f64 * self.extra_cost.max(0.)))
    }
}
