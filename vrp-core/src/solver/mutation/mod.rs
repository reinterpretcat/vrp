//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use std::sync::Arc;

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

mod infeasible_search;
pub use self::infeasible_search::InfeasibleSearch;

mod local_search;
pub use self::local_search::LocalSearch;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

/// A trait which defines mutation behavior.
pub trait Mutation {
    /// Mutates passed insertion context.
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext;
}

/// Provides the way to pick one mutation from the group of mutation methods.
pub struct WeightedMutation {
    mutations: Vec<Arc<dyn Mutation + Send + Sync>>,
    weights: Vec<usize>,
}

impl WeightedMutation {
    /// Creates a new instance of `WeightedMutation`.
    pub fn new(mutations: Vec<Arc<dyn Mutation + Send + Sync>>, weights: Vec<usize>) -> Self {
        Self { mutations, weights }
    }
}

impl Mutation for WeightedMutation {
    fn mutate(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());

        self.mutations[index].mutate(refinement_ctx, insertion_ctx)
    }
}
