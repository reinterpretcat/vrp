#[cfg(test)]
#[path = "../../../tests/unit/refinement/termination/max_generation_test.rs"]
mod max_generation_test;

use crate::models::common::ObjectiveCost;
use crate::models::Solution;
use crate::refinement::termination::TerminationCriteria;
use crate::refinement::RefinementContext;
use std::sync::Arc;

/// Stops when maximum amount of generations is exceeded.
pub struct MaxGeneration {
    limit: usize,
}

impl MaxGeneration {
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

impl Default for MaxGeneration {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl TerminationCriteria for MaxGeneration {
    fn is_termination(
        &self,
        refinement_ctx: &RefinementContext,
        solution: (Arc<Solution>, ObjectiveCost, bool),
    ) -> bool {
        refinement_ctx.generation > self.limit
    }
}
