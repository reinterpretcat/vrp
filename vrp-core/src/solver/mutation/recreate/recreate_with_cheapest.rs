use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;

/// A recreate method which is equivalent to cheapest insertion heuristic.
pub struct RecreateWithCheapest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self::new(
            Box::new(AllJobSelector::default()),
            Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
            )),
        )
    }
}

impl RecreateWithCheapest {
    /// Creates a new instance of `RecreateWithCheapest`.
    pub fn new(
        job_selector: Box<dyn JobSelector + Send + Sync>,
        job_reducer: Box<dyn JobMapReducer + Send + Sync>,
    ) -> Self {
        Self { job_selector, job_reducer }
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
