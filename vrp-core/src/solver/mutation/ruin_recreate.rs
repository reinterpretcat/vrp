///! Contains a mutation operator based on ruin and recreate principle.
use super::*;

/// A mutation operator based on ruin and recreate principle.
pub struct RuinAndRecreate {
    ruin: Box<dyn Ruin + Send + Sync>,
    recreate: Box<dyn Recreate + Send + Sync>,
}

impl RuinAndRecreate {
    /// Creates a new instance of `RuinAndRecreate` using given ruin and recreate methods.
    pub fn new(recreate: Box<dyn Recreate + Send + Sync>, ruin: Box<dyn Ruin + Send + Sync>) -> Self {
        Self { recreate, ruin }
    }
}

impl Mutation for RuinAndRecreate {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, self.ruin.run(refinement_ctx, insertion_ctx.deep_copy()))
    }
}
