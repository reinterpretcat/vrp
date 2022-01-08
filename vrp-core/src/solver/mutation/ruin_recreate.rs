///! Contains a mutation operator based on ruin and recreate principle.
use super::*;
use crate::construction::heuristics::finalize_insertion_ctx;
use rosomaxa::heuristics::HeuristicSolution;
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

impl Mutation for RuinAndRecreate {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let mut insertion_ctx =
            self.recreate.run(refinement_ctx, self.ruin.run(refinement_ctx, insertion_ctx.deep_copy()));

        finalize_insertion_ctx(&mut insertion_ctx);

        insertion_ctx
    }
}
