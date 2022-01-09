use crate::construction::heuristics::InsertionContext;
use crate::solver::search::LocalOperator;
use crate::solver::RefinementContext;
use rosomaxa::heuristics::HeuristicSolution;
use rosomaxa::prelude::HeuristicOperator;
use std::sync::Arc;

/// A mutation operator which applies local search principles.
pub struct LocalSearch {
    operator: Arc<dyn LocalOperator + Send + Sync>,
}

impl LocalSearch {
    /// Creates a new instance of `LocalSearch`.
    pub fn new(operator: Arc<dyn LocalOperator + Send + Sync>) -> Self {
        Self { operator }
    }
}

impl HeuristicOperator for LocalSearch {
    type Context = RefinementContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        if let Some(new_insertion_ctx) = self.operator.explore(refinement_ctx, insertion_ctx) {
            new_insertion_ctx
        } else {
            insertion_ctx.deep_copy()
        }
    }
}
