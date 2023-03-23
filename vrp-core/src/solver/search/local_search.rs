use crate::construction::heuristics::InsertionContext;
use crate::models::GoalContext;
use crate::solver::search::LocalOperator;
use crate::solver::RefinementContext;
use rosomaxa::prelude::*;
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

impl HeuristicSearchOperator for LocalSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
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
