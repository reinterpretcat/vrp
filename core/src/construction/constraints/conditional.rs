#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/conditional_test.rs"]
mod conditional_test;

use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::construction::states::{RouteContext, SolutionContext};
use crate::models::problem::Job;
use hashbrown::HashSet;
use std::slice::Iter;
use std::sync::Arc;

pub type JobCondition = Box<dyn Fn(&SolutionContext, &Arc<Job>) -> bool + Send + Sync>;

/// Allows to promote jobs between required and ignored collection using some condition.
/// Useful to model some optional/conditional activities, e.g. breaks, refueling, etc.
pub struct ConditionalJobModule {
    required_condition: Option<JobCondition>,
    locked_condition: Option<JobCondition>,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConditionalJobModule {
    pub fn new(required_condition: Option<JobCondition>, locked_condition: Option<JobCondition>) -> Self {
        Self { required_condition, locked_condition, state_keys: vec![], constraints: vec![] }
    }
}

impl ConstraintModule for ConditionalJobModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Arc<Job>) {
        // TODO avoid calling this on each insertion as it is expensive.
        self.accept_solution_state(solution_ctx);
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        if let Some(required_condition) = &self.required_condition {
            // identify ignored inside required
            let ignored: HashSet<Arc<Job>> =
                ctx.required.iter().filter(|job| !(required_condition)(ctx, job)).cloned().collect();
            ctx.required.retain(|job| !ignored.contains(job));

            // identify required inside ignored
            let required: HashSet<Arc<Job>> =
                ctx.ignored.iter().filter(|job| (required_condition)(ctx, job)).cloned().collect();
            ctx.ignored.retain(|job| !required.contains(job));

            ctx.required.extend(required);
            ctx.ignored.extend(ignored);
        }

        if let Some(locked_condition) = &self.locked_condition {
            // remove from locked
            let not_locked: HashSet<Arc<Job>> =
                ctx.locked.iter().filter(|job| !(locked_condition)(ctx, job)).cloned().collect();
            ctx.locked.retain(|job| !not_locked.contains(job));

            // promote to locked
            let locked: HashSet<Arc<Job>> =
                ctx.required.iter().filter(|job| (locked_condition)(ctx, job)).cloned().collect();
            ctx.locked.extend(locked);
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}
