///! Contains a mutation operator based on ruin and recreate principle.
use super::*;
use crate::construction::heuristics::finalize_insertion_ctx;
use crate::models::problem::ProblemObjective;
use rosomaxa::HeuristicSolution;
use std::sync::Arc;

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreate {
    ruin: Arc<dyn Ruin + Send + Sync>,
    recreate: Arc<dyn Recreate + Send + Sync>,
}

impl RuinAndRecreate {
    /// Creates a new instance of `RuinAndRecreate` using given ruin and recreate methods.
    pub fn new(ruin: Arc<dyn Ruin + Send + Sync>, recreate: Arc<dyn Recreate + Send + Sync>) -> Self {
        Self { ruin, recreate }
    }
}

impl HeuristicSearchOperator for RuinAndRecreate {
    type Context = RefinementContext;
    type Objective = ProblemObjective;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        let insertion_ctx = solution;

        let mut insertion_ctx =
            self.recreate.run(refinement_ctx, self.ruin.run(refinement_ctx, insertion_ctx.deep_copy()));

        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}
