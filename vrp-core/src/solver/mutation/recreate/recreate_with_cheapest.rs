use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;

/// A recreate method which is equivalent to cheapest insertion heuristic.
pub struct RecreateWithCheapest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    insertion_heuristic: InsertionHeuristic,
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            route_selector: Box::new(AllRouteSelector::default()),
            result_selector: Box::new(BestResultSelector::default()),
            insertion_heuristic: Default::default(),
        }
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.insertion_heuristic.process(
            insertion_ctx,
            self.job_selector.as_ref(),
            self.route_selector.as_ref(),
            self.result_selector.as_ref(),
            &refinement_ctx.quota,
        )
    }
}
