#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/conditional_test.rs"]
mod conditional_test;

use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::construction::states::{RouteContext, SolutionContext};
use crate::models::problem::Job;
use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;

/// Allows to promote jobs between required and ignored collection using some condition.
/// Useful to model some optional/conditional activities, e.g. breaks, refueling, etc.
pub struct ConditionalJobModule {
    required_condition: Box<dyn Fn(&SolutionContext, &Arc<Job>) -> bool + Send + Sync>,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConditionalJobModule {
    pub fn new(condition: Box<dyn Fn(&SolutionContext, &Arc<Job>) -> bool + Send + Sync>) -> Self {
        Self { required_condition: condition, state_keys: vec![], constraints: vec![] }
    }
}

impl ConstraintModule for ConditionalJobModule {
    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // identify ignored inside required
        let ignored: HashSet<Arc<Job>> =
            ctx.required.iter().filter(|job| !(self.required_condition)(ctx, job)).cloned().collect();
        ctx.required.retain(|job| !ignored.contains(job));

        // identify required inside ignored
        let required: HashSet<Arc<Job>> =
            ctx.ignored.iter().filter(|job| (self.required_condition)(ctx, job)).cloned().collect();
        ctx.ignored.retain(|job| !required.contains(job));

        ctx.required.extend(required);
        ctx.ignored.extend(ignored);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}
