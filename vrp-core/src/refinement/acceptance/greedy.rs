#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::refinement::acceptance::Acceptance;
use crate::refinement::{Individuum, RefinementContext};
use std::cmp::Ordering::Less;

/// Greedy acceptance which accepts only solutions with less objective costs.
pub struct Greedy {}

impl Default for Greedy {
    fn default() -> Self {
        Self {}
    }
}

impl Acceptance for Greedy {
    fn is_accepted(&self, refinement_ctx: &mut RefinementContext, solution: &Individuum) -> bool {
        let new = &solution.1;
        let best = refinement_ctx.population.best().map(|(_, cost, _)| cost);

        match best {
            Some(best) => new.cmp_relaxed(&best).0 == Less,
            _ => true,
        }
    }
}
