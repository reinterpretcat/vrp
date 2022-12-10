//! This module contains various Local Search operators.

use crate::construction::heuristics::*;
use crate::solver::RefinementContext;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::sync::Arc;

mod exchange_2opt;
pub use self::exchange_2opt::*;

mod exchange_inter_route;
pub use self::exchange_inter_route::*;

mod exchange_intra_route;
pub use self::exchange_intra_route::*;

mod exchange_sequence;
pub use self::exchange_sequence::*;

mod exchange_swap_star;
pub use self::exchange_swap_star::*;

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
                if refinement_ctx.problem.goal.total_order(insertion_ctx, &new_result) == Ordering::Greater {
                    return Some(new_result);
                } else {
                    old_result = new_result;
                }
            }
        }

        Some(old_result)
    }
}

/// Applies insertion success by creating a new route context from it.
fn apply_insertion(insertion_ctx: &mut InsertionContext, success: InsertionSuccess) {
    let route_index =
        insertion_ctx.solution.routes.iter().position(|ctx| ctx.route.actor == success.context.route.actor).unwrap();

    // NOTE replace existing route context with the different
    insertion_ctx.solution.routes[route_index] =
        RouteContext::new_with_state(success.context.route.clone(), success.context.state.clone());

    apply_insertion_success(insertion_ctx, success)
}

fn get_random_route_idx(insertion_ctx: &InsertionContext) -> Option<usize> {
    let routes = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter_map(|(idx, rc)| if rc.route.tour.job_count() > 1 { Some(idx) } else { None })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        None
    } else {
        Some(routes[insertion_ctx.environment.random.uniform_int(0, (routes.len() - 1) as i32) as usize])
    }
}
