//! Contains a mutation operator based on ruin and recreate principle.

use super::*;
use crate::construction::heuristics::finalize_insertion_ctx;
use crate::models::GoalContext;
use rosomaxa::HeuristicSolution;
use std::sync::Arc;

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreate {
    ruin: Arc<dyn Ruin>,
    recreate: Arc<dyn Recreate>,
}

impl RuinAndRecreate {
    /// Creates a new instance of `RuinAndRecreate` using given ruin and recreate methods.
    pub fn new(ruin: Arc<dyn Ruin>, recreate: Arc<dyn Recreate>) -> Self {
        Self { ruin, recreate }
    }
}

impl HeuristicSearchOperator for RuinAndRecreate {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = self.ruin.run(refinement_ctx, solution.deep_copy());
        // Recreate owns the partial-solution boundary: insertion-based implementations restore state
        // in `prepare_insertion_ctx` before any selection or evaluation. DummyRecreate performs no
        // analysis, and the finalization below restores its result.
        let mut insertion_ctx = self.recreate.run(refinement_ctx, insertion_ctx);

        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}
