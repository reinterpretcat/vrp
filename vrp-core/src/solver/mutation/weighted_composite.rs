use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::Mutation;
use crate::solver::RefinementContext;
use crate::utils::parallel_into_collect;
use std::sync::Arc;

/// A mutation operator which uses others based on their weight probability.
pub struct WeightedComposite {
    inners: Vec<Arc<dyn Mutation + Send + Sync>>,
    weights: Vec<usize>,
}

impl WeightedComposite {
    /// Creates a new instance of `WeightedComposite`.
    pub fn new(inners: Vec<(Arc<dyn Mutation + Send + Sync>, usize)>) -> Self {
        let weights = inners.iter().map(|(_, weight)| *weight).collect();
        let inners = inners.into_iter().map(|(inners, _)| inners).collect();
        Self { inners, weights }
    }
}

impl Mutation for WeightedComposite {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.random.weighted(self.weights.as_slice());

        self.inners[index].mutate_one(refinement_ctx, insertion_ctx)
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<InsertionContext>,
    ) -> Vec<InsertionContext> {
        parallel_into_collect(individuals, |insertion_ctx| self.mutate_one(refinement_ctx, insertion_ctx))
    }
}
