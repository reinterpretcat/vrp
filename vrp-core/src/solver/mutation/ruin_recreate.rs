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

    /// Creates a new instance of `RuinAndRecreate` using default ruin and recreate methods.
    pub fn new_from_problem(problem: Arc<Problem>) -> Self {
        Self {
            recreate: Box::new(CompositeRecreate::new_from_problem(problem.clone())),
            ruin: Box::new(CompositeRuin::new_from_problem(problem)),
        }
    }
}

impl Mutation for RuinAndRecreate {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, self.ruin.run(refinement_ctx, insertion_ctx.deep_copy()))
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<&InsertionContext>,
    ) -> Vec<InsertionContext> {
        parallel_into_collect(individuals.iter().enumerate().collect(), |(idx, insertion_ctx)| {
            refinement_ctx
                .environment
                .parallelism
                .thread_pool_execute(idx, || self.mutate_one(refinement_ctx, insertion_ctx))
        })
    }
}
