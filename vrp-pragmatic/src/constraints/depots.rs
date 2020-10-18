use std::slice::Iter;
use vrp_core::construction::constraints::{ConstraintModule, ConstraintVariant};
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::problem::Job;

pub struct DepotModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl Default for DepotModule {
    fn default() -> Self {
        Self { constraints: vec![], keys: vec![] }
    }
}

impl ConstraintModule for DepotModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {
        // TODO remove routes with only depots
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}
