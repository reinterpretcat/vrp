//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::utils::{parallel_into_collect, unwrap_from_result, Random};

mod local;
pub use self::local::*;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

mod utils;
pub(crate) use self::utils::*;

mod decompose_search;
pub use self::decompose_search::DecomposeSearch;

mod local_search;
pub use self::local_search::LocalSearch;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

use crate::algorithms::nsga2::Objective;
use crate::models::Problem;
use std::cmp::Ordering;
use std::sync::Arc;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Mutates passed insertion context.
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext;

    /// Mutates passed insertion contexts.
    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<&InsertionContext>,
    ) -> Vec<InsertionContext>;
}

/// A type which specifies probability behavior for mutation selection.
pub type MutationProbability = Box<dyn Fn(&RefinementContext, &InsertionContext) -> bool + Send + Sync>;

/// A type which specifies a group of multiple mutation strategies with their probability.
pub type MutationGroup = (Vec<(Arc<dyn Mutation + Send + Sync>, MutationProbability)>, usize);

/// A mutation operator which uses others based on their weight probability.
pub struct CompositeMutation {
    inners: Vec<Vec<(Arc<dyn Mutation + Send + Sync>, MutationProbability)>>,
    weights: Vec<usize>,
}

impl CompositeMutation {
    /// Creates a new instance of `CompositeMutation`.
    pub fn new(inners: Vec<MutationGroup>) -> Self {
        let (inners, weights) = inners.into_iter().unzip();

        Self { inners, weights }
    }
}

impl Mutation for CompositeMutation {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        let objective = &refinement_ctx.problem.objective;

        unwrap_from_result(
            self.inners[index].iter().filter(|(_, probability)| probability(refinement_ctx, insertion_ctx)).try_fold(
                insertion_ctx.deep_copy(),
                |ctx, (mutation, _)| {
                    let new_insertion_ctx = mutation.mutate_one(refinement_ctx, &ctx);

                    if objective.total_order(&insertion_ctx, &new_insertion_ctx) == Ordering::Greater {
                        // NOTE exit immediately as we don't want to lose improvement from original individual
                        Err(new_insertion_ctx)
                    } else {
                        Ok(new_insertion_ctx)
                    }
                },
            ),
        )
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

/// Creates a mutation probability which uses `is_hit` method from passed random object.
pub fn create_scalar_mutation_probability(
    scalar_probability: f64,
    random: Arc<dyn Random + Send + Sync>,
) -> MutationProbability {
    Box::new(move |_, _| random.is_hit(scalar_probability))
}
