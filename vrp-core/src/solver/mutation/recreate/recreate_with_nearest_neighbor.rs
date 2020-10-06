use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::Recreate;
use crate::solver::RefinementContext;

/// A recreate strategy which solution using nearest neighbor algorithm.
pub struct RecreateWithNearestNeighbor {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithNearestNeighbor {
    fn default() -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
            )),
        }
    }
}

impl Recreate for RecreateWithNearestNeighbor {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::new(InsertionPosition::Last).process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
