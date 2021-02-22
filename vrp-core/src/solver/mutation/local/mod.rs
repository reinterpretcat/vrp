//! This module contains various Local Search operators.

use crate::algorithms::nsga2::Objective;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use std::cmp::Ordering;
use std::sync::Arc;

mod exchange_inter_route;
pub use self::exchange_inter_route::*;

mod exchange_intra_route;
pub use self::exchange_intra_route::*;

mod reschedule_departure;
pub use self::reschedule_departure::*;

/// Specifies behavior of a local search operator.
pub trait LocalOperator {
    /// Applies local search operator to passed solution in order to explore possible
    /// small move in solution space which leads to a different solution.
    fn explore(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext)
        -> Option<InsertionContext>;
}

/// Provides the way to run multiple local search operators with different probability.
pub struct CompositeLocalOperator {
    operators: Vec<Arc<dyn LocalOperator + Send + Sync>>,
    weights: Vec<usize>,
    times: (i32, i32),
}

impl CompositeLocalOperator {
    /// Creates a new instance of `CompositeLocalOperator`.
    pub fn new(operators: Vec<(Arc<dyn LocalOperator + Send + Sync>, usize)>, min: usize, max: usize) -> Self {
        let weights = operators.iter().map(|(_, weight)| *weight).collect();
        let operators = operators.into_iter().map(|(operator, _)| operator).collect();

        Self { operators, weights, times: (min as i32, max as i32) }
    }
}

impl LocalOperator for CompositeLocalOperator {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let random = insertion_ctx.environment.random.as_ref();
        let times = random.uniform_int(self.times.0, self.times.1);

        let mut old_result = insertion_ctx.deep_copy();

        for _ in 0..times {
            let index = random.weighted(self.weights.as_slice());
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
