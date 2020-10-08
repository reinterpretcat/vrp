//! This module contains various Local Search operators.

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

mod exchange_inter_route;
pub use self::exchange_inter_route::ExchangeInterRouteBest;
pub use self::exchange_inter_route::ExchangeInterRouteRandom;

/// Specifies behavior of a local search operator.
pub trait LocalSearch {
    /// Applies local search operator to passed solution in order to explore possible
    /// small move in solution space which leads to improved or equal, but different solution.
    fn try_improve(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext>;

    /// Applies local search operator to passed solution in order to explore possible
    /// small move in solution space which leads to a different, possible worse, solution.
    fn try_diversify(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext>;
}
