//! Contains a selection which uses naive approach to select parents for offspring.

#[cfg(test)]
#[path = "../../../tests/unit/solver/selection/naive_selection_test.rs"]
mod naive_selection_test;

use super::*;
use std::iter::once;

/// A naive selection algorithm.
pub struct NaiveSelection {
    offspring_size: usize,
}

impl NaiveSelection {
    /// Creates a new instance of `NaiveSelection`.
    pub fn new(offspring_size: usize) -> Self {
        Self { offspring_size }
    }
}

impl Selection for NaiveSelection {
    fn select_parents<'a>(&self, refinement_ctx: &'a RefinementContext) -> Vec<&'a InsertionContext> {
        assert!(refinement_ctx.population.size() > 0);
        let random = refinement_ctx.population.nth(0).unwrap().random.clone();

        once(0_usize)
            .chain(
                (1..self.offspring_size)
                    .map(|_| random.uniform_int(0, refinement_ctx.population.size() as i32 - 1) as usize),
            )
            .take(self.offspring_size)
            .filter_map(|idx| refinement_ctx.population.nth(idx))
            .map(|individual| individual)
            .collect()
    }
}
