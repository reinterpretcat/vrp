//! This module contains various Local Search operators.

mod inter_route_exchange;
pub use self::inter_route_exchange::InterRouteExchange;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

/// Specifies behavior of a local search operator.
pub trait LocalSearch {
    /// Applies local search operator to passed solution in order to explore possible
    /// small moves in solution space.
    fn explore(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}
