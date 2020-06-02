#[cfg(test)]
#[path = "../../../tests/unit/solver/termination/max_generation_test.rs"]
mod max_generation_test;

use crate::solver::termination::Termination;
use crate::solver::RefinementContext;

/// A termination criteria which is in terminated state when maximum amount of generations is exceeded.
pub struct MaxGeneration {
    limit: usize,
}

impl MaxGeneration {
    /// Creates a new instance of `MaxGeneration`.
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

impl Termination for MaxGeneration {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext) -> bool {
        refinement_ctx.generation > self.limit
    }
}
