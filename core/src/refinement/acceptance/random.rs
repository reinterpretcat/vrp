#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::RefinementContext;

/// Accepts some solutions randomly given probability.
pub struct RandomProbability {
    other: Box<dyn Acceptance>,
    probability: f64,
}

impl RandomProbability {
    pub fn new(other: Box<dyn Acceptance>, probability: f64) -> Self {
        Self { other, probability }
    }
}

impl Default for RandomProbability {
    fn default() -> Self {
        Self::new(Box::new(Greedy::default()), 0.001)
    }
}

impl Acceptance for RandomProbability {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost)) -> bool {
        let random = solution.0.random.clone();

        self.other.is_accepted(refinement_ctx, solution) || self.probability > random.uniform_real(0., 1.)
    }
}
