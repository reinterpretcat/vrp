//! This module contains various Local Search operators.

use crate::algorithms::nsga2::Objective;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use std::cmp::Ordering;

mod exchange_inter_route;
pub use self::exchange_inter_route::ExchangeInterRouteBest;
pub use self::exchange_inter_route::ExchangeInterRouteRandom;

mod exchange_intra_route;
pub use self::exchange_intra_route::ExchangeIntraRouteRandom;

/// Specifies behavior of a local search operator.
pub trait LocalSearch {
    /// Applies local search operator to passed solution in order to explore possible
    /// small move in solution space which leads to a different solution.
    fn explore(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext)
        -> Option<InsertionContext>;
}

/// Provides the way to run multiple local search operators with different probability.
pub struct CompositeLocalSearch {
    operators: Vec<Box<dyn LocalSearch + Send + Sync>>,
    weights: Vec<usize>,
    times: (i32, i32),
}

impl CompositeLocalSearch {
    /// Creates a new instance of `CompositeLocalSearch`.
    pub fn new(operators: Vec<(Box<dyn LocalSearch + Send + Sync>, usize)>, min: usize, max: usize) -> Self {
        let weights = operators.iter().map(|(_, weight)| *weight).collect();
        let operators = operators.into_iter().map(|(operator, _)| operator).collect();

        Self { operators, weights, times: (min as i32, max as i32) }
    }
}

impl Default for CompositeLocalSearch {
    fn default() -> Self {
        Self::new(
            vec![
                (Box::new(ExchangeInterRouteBest::default()), 100),
                (Box::new(ExchangeInterRouteRandom::default()), 30),
                (Box::new(ExchangeIntraRouteRandom::default()), 30),
            ],
            1,
            4,
        )
    }
}

impl LocalSearch for CompositeLocalSearch {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let times = insertion_ctx.random.uniform_int(self.times.0, self.times.1);

        let mut old_result = insertion_ctx.deep_copy();

        for _ in 0..times {
            let index = insertion_ctx.random.weighted(self.weights.as_slice());
            let new_result = self.operators.get(index).unwrap().explore(refinement_ctx, &old_result);

            if let Some(new_result) = new_result {
                if refinement_ctx.problem.objective.total_order(insertion_ctx, &new_result) == Ordering::Greater {
                    return Some(new_result);
                } else {
                    old_result = new_result;
                }
            }
        }

        Some(old_result)
    }
}
