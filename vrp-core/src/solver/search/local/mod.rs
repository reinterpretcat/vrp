//! This module contains various Local Search operators.

use crate::construction::heuristics::*;
use crate::solver::RefinementContext;
use rosomaxa::prelude::*;
use rosomaxa::utils::SelectionSamplingIterator;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

mod exchange_inter_route;
pub use self::exchange_inter_route::*;

mod exchange_intra_route;
pub use self::exchange_intra_route::*;

mod relocate_inter_route;
pub use self::relocate_inter_route::*;

mod exchange_sequence;
pub use self::exchange_sequence::*;

mod exchange_swap_star;
pub use self::exchange_swap_star::*;

mod reschedule_departure;
pub use self::reschedule_departure::*;

/// Specifies behavior of a local search operator.
pub trait LocalOperator: Send + Sync {
    /// Applies local search operator to passed solution in order to explore possible
    /// small move in solution space which leads to a different solution.
    fn explore(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext)
    -> Option<InsertionContext>;
}

/// Provides the way to run multiple local search operators with different probability.
pub struct CompositeLocalOperator {
    operators: Vec<Arc<dyn LocalOperator>>,
    weights: Vec<usize>,
    times: (i32, i32),
}

impl CompositeLocalOperator {
    /// Creates a new instance of `CompositeLocalOperator`.
    pub fn new(operators: Vec<(Arc<dyn LocalOperator>, usize)>, min: usize, max: usize) -> Self {
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
fn apply_insertion_with_route(insertion_ctx: &mut InsertionContext, result: (InsertionSuccess, Option<RouteContext>)) {
    let (success, route_ctx) = result;

    if let Some(route_ctx) = route_ctx {
        debug_assert!(success.actor == route_ctx.route().actor);

        let route_index = insertion_ctx
            .solution
            .routes
            .iter()
            .position(|route_ctx| route_ctx.route().actor == success.actor)
            .unwrap();

        // NOTE replace existing route with a new non empty route
        insertion_ctx.solution.routes[route_index] = route_ctx;
    }

    apply_insertion_success(insertion_ctx, success)
}

/// Creates candidate route pairs for granular inter-route local search.
fn create_route_pairs(insertion_ctx: &InsertionContext, route_pairs_threshold: usize) -> Vec<(usize, usize)> {
    let random = insertion_ctx.environment.random.clone();

    if random.is_hit(0.1) {
        let route_count = insertion_ctx.solution.routes.len();
        // NOTE this is needed to have size hint properly set
        let all_route_pairs = (0..route_count)
            .flat_map(move |outer_idx| {
                (0..route_count)
                    .filter(move |&inner_idx| outer_idx > inner_idx)
                    .map(move |inner_idx| (outer_idx, inner_idx))
            })
            .collect::<Vec<_>>();

        SelectionSamplingIterator::new(all_route_pairs.into_iter(), route_pairs_threshold, random).collect()
    } else {
        let route_groups = group_routes_by_proximity(insertion_ctx);
        let used_indices = RefCell::new(HashSet::<(usize, usize)>::new());
        let distances = route_groups
            .into_iter()
            .enumerate()
            .flat_map(|(outer_idx, route_group)| {
                route_group
                    .into_iter()
                    .filter(|inner_idx| {
                        let used_indices = used_indices.borrow();
                        !used_indices.contains(&(outer_idx, *inner_idx))
                            && !used_indices.contains(&(*inner_idx, outer_idx))
                    })
                    .inspect(|inner_idx| {
                        let mut used_indices = used_indices.borrow_mut();
                        used_indices.insert((outer_idx, *inner_idx));
                        used_indices.insert((*inner_idx, outer_idx));
                    })
                    .next()
                    .map(|inner_idx| (outer_idx, inner_idx))
            })
            .collect::<Vec<_>>();

        SelectionSamplingIterator::new(distances.into_iter(), route_pairs_threshold, random).collect()
    }
}
