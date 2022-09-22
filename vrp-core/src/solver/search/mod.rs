//! The mutation module specifies building blocks for mutation operator used by evolution.
//!
//! The default implementation of mutation operator is `RuinAndRecreateMutation` which is based on
//! **ruin and recreate** principle, introduced by [`Schrimpf et al. (2000)`].
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!

use crate::construction::heuristics::InsertionContext;
use crate::models::GoalContext;
use crate::solver::{RefinementContext, TargetSearchOperator};
use rosomaxa::hyper::HeuristicDiversifyOperator;
use rosomaxa::prelude::HeuristicSearchOperator;

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

mod redistribute_search;
pub use self::redistribute_search::RedistributeSearch;

mod ruin_recreate;
pub use self::ruin_recreate::RuinAndRecreate;

/// Provides the way to pick one heuristic operator from the group.
pub struct WeightedHeuristicOperator {
    mutations: Vec<TargetSearchOperator>,
    weights: Vec<usize>,
}

impl WeightedHeuristicOperator {
    /// Creates a new instance of `WeightedHeuristicOperator`.
    pub fn new(mutations: Vec<TargetSearchOperator>, weights: Vec<usize>) -> Self {
        Self { mutations, weights }
    }
}

impl HeuristicSearchOperator for WeightedHeuristicOperator {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let index = solution.environment.random.weighted(self.weights.as_slice());

        self.mutations[index].search(heuristic_ctx, solution)
    }
}

impl HeuristicDiversifyOperator for WeightedHeuristicOperator {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        vec![self.search(heuristic_ctx, solution)]
    }
}
