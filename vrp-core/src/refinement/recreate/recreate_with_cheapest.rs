use crate::construction::heuristics::*;
use crate::construction::states::InsertionContext;
use crate::refinement::recreate::Recreate;
use crate::refinement::RefinementContext;

/// A recreate method which is equivalent to cheapest insertion heuristic.
pub struct RecreateWithCheapest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(PairJobMapReducer::new(Box::new(BestResultSelector::default()))),
        }
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, _refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::process(&self.job_selector, &self.job_reducer, insertion_ctx)
    }
}
