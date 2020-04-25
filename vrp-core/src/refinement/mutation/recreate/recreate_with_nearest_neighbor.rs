use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::refinement::mutation::Recreate;
use crate::refinement::RefinementContext;

/// Recreates solution using nearest neighbor algorithm.
pub struct RecreateWithNearestNeighbor {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithNearestNeighbor {
    fn default() -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(PairJobMapReducer::new(Box::new(BestResultSelector::default()))),
        }
    }
}

impl Recreate for RecreateWithNearestNeighbor {
    fn run(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::new(InsertionPosition::Last).process(
            &self.job_selector,
            &self.job_reducer,
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
