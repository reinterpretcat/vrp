#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::models::common::ObjectiveCost;
use crate::models::Solution;
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

impl Acceptance for Greedy {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (Arc<Solution>, ObjectiveCost)) -> bool {
        match refinement_ctx.population.first() {
            Some(best) => solution.1.total() < best.1.total(),
            None => true,
        }
    }
}
