use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::LocalSearch;
use crate::solver::RefinementContext;
use std::ops::Range;

/// A local search operator which tries to exchange jobs between routes.
pub struct InterRouteExchange {
    _exchange_job_range: Range<usize>,
}

impl InterRouteExchange {
    /// Creates a new instance of `InterRouteExchange`.
    pub fn new(exchange_range: Range<usize>) -> Self {
        Self { _exchange_job_range: exchange_range }
    }
}

impl Default for InterRouteExchange {
    fn default() -> Self {
        Self { _exchange_job_range: 1..2 }
    }
}

impl LocalSearch for InterRouteExchange {
    fn explore(&self, _refinement_ctx: &RefinementContext, _insertion_ctx: InsertionContext) -> InsertionContext {
        unimplemented!()
    }
}
