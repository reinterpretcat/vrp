#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::RefinementContext;

/// Accepts some solutions randomly.
pub struct SmoothRandom {
    other: Box<dyn Acceptance>,
    max: f64,
    generations: f64,
}

impl SmoothRandom {
    pub fn new(other: Box<dyn Acceptance>, max: f64, generations: f64) -> Self {
        Self { other, max, generations }
    }

    fn probability(&self, x: f64) -> f64 {
        self.max - (-1. * 2_f64.ln() * x / self.generations).exp()
    }
}

impl Default for SmoothRandom {
    fn default() -> Self {
        Self::new(Box::new(Greedy::default()), 0.01, 1000.)
    }
}

impl Acceptance for SmoothRandom {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost)) -> bool {
        let random = solution.0.random.clone();

        self.other.is_accepted(refinement_ctx, solution)
            || self.probability(refinement_ctx.generation as f64) > random.uniform_real(0., 1.)
    }
}
