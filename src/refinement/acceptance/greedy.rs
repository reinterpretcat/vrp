#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::Acceptance;
use crate::refinement::RefinementContext;
use std::sync::Arc;

/// Greedy acceptance which accepts only better solutions.
pub struct Greedy {}

impl Greedy {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Greedy {
    fn default() -> Self {
        Self::new()
    }
}

impl Acceptance for Greedy {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost)) -> bool {
        match refinement_ctx.population.first() {
            Some(best) => solution.1.total() < best.1.total(),
            None => true,
        }
    }
}
