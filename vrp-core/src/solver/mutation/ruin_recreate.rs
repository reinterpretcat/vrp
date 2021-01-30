///! Contains a mutation operator based on ruin and recreate principle.
use super::*;
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
        self.recreate.run(refinement_ctx, self.ruin.run(refinement_ctx, insertion_ctx.deep_copy()))
    }
}
