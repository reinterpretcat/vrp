#[cfg(test)]
#[path = "../../../tests/unit/refinement/termination/max_generation_test.rs"]
mod max_generation_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::termination::Termination;
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

impl Termination for MaxGeneration {
    fn is_termination(&self, refinement_ctx: &RefinementContext, _: (&InsertionContext, ObjectiveCost, bool)) -> bool {
        refinement_ctx.generation > self.limit
    }
}
