//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::utils::parallel_into_collect;

mod local;
pub use self::local::*;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

mod utils;
pub(crate) use self::utils::*;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

use crate::models::Problem;
use std::sync::Arc;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Mutates passed insertion context.
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;

    /// Mutates passed insertion contexts.
    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<InsertionContext>,
    ) -> Vec<InsertionContext>;
}

/// A mutation operator which uses others based on their weight probability.
pub struct CompositeMutation {
    inners: Vec<Arc<dyn Mutation + Send + Sync>>,
    weights: Vec<usize>,
}

impl CompositeMutation {
    /// Creates a new instance of `WeightedComposite`.
    pub fn new(inners: Vec<(Arc<dyn Mutation + Send + Sync>, usize)>) -> Self {
        let weights = inners.iter().map(|(_, weight)| *weight).collect();
        let inners = inners.into_iter().map(|(inners, _)| inners).collect();
        Self { inners, weights }
    }
}

impl Mutation for CompositeMutation {
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
