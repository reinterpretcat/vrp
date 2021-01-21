use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::{LocalOperator, Mutation};
use crate::solver::RefinementContext;
use crate::utils::parallel_into_collect;

/// A mutation operator which applies local search principles.
pub struct LocalSearch {
    operator: Box<dyn LocalOperator + Send + Sync>,
}

impl LocalSearch {
    /// Creates a new instance of `LocalSearch`.
    pub fn new(operator: Box<dyn LocalOperator + Send + Sync>) -> Self {
        Self { operator }
    }
}

impl Mutation for LocalSearch {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        if let Some(new_insertion_ctx) = self.operator.explore(refinement_ctx, insertion_ctx) {
            new_insertion_ctx
        } else {
            insertion_ctx.deep_copy()
        }
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<&InsertionContext>,
    ) -> Vec<InsertionContext> {
        parallel_into_collect(
            individuals,
            refinement_ctx.environment.parallelism.outer_degree.clone(),
            |insertion_ctx| self.mutate_one(refinement_ctx, insertion_ctx),
        )
    }
}
