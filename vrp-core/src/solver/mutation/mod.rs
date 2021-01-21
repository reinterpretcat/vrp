//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::utils::{parallel_into_collect, unwrap_from_result};

mod local;
pub use self::local::*;

mod recreate;
pub use self::recreate::*;

mod ruin;
pub use self::ruin::*;

mod utils;
pub(crate) use self::utils::*;

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

/// A type which specifies a group of multiple mutation strategies with their probability.
pub type MutationGroup = (Vec<(Arc<dyn Mutation + Send + Sync>, f64)>, usize);

/// A mutation operator which uses others based on their weight probability.
pub struct CompositeMutation {
    inners: Vec<Vec<(Arc<dyn Mutation + Send + Sync>, f64)>>,
    weights: Vec<usize>,
}

impl CompositeMutation {
    /// Creates a new instance of `CompositeMutation`.
    pub fn new(inners: Vec<MutationGroup>) -> Self {
        let weights = inners.iter().map(|(_, weight)| *weight).collect();
        let inners = inners.into_iter().map(|(inner, _)| inner).collect();

        Self { inners, weights }
    }
}

impl Mutation for CompositeMutation {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let random = insertion_ctx.environment.random.clone();
        let index = random.weighted(self.weights.as_slice());
        let objective = &refinement_ctx.problem.objective;

        unwrap_from_result(self.inners[index].iter().filter(|(_, probability)| random.is_hit(*probability)).try_fold(
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
        ))
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
