use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;

struct SubsetRouteSelector {}

impl RouteSelector for SubsetRouteSelector {
    fn select<'a>(&'a self, _ctx: &'a InsertionContext, _job: &Job) -> Box<dyn Iterator<Item = RouteContext>> {
        unimplemented!()
    }
}

/// Evaluates insertion of
pub struct RecreateWithSubset {
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithSubset {
    fn default() -> Self {
        Self {
            route_selector: Box::new(SubsetRouteSelector {}),
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(PairJobMapReducer::new(Box::new(BestResultSelector::default()))),
        }
    }
}

impl Recreate for RecreateWithSubset {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.route_selector.as_ref(),
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
