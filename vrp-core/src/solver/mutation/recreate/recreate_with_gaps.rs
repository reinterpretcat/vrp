use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;
use rand::prelude::*;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        // TODO we should prefer to always insert locked jobs
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        // TODO improve formula
        let max_jobs = self.min_jobs.max(ctx.solution.required.len());
        let take_jobs = ctx.environment.random.uniform_int(self.min_jobs as i32, max_jobs as i32) as usize;

        Box::new(ctx.solution.required.iter().take(take_jobs).cloned())
    }
}

/// A recreate method which selects on each insertion step only subset of randomly chosen jobs.
pub struct RecreateWithGaps {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    insertion_heuristic: InsertionHeuristic,
}

impl RecreateWithGaps {
    /// Creates a new instance of `RecreateWithGaps`.
    pub fn new(min_jobs: usize) -> Self {
        Self {
            job_selector: Box::new(GapsJobSelector { min_jobs }),
            route_selector: Box::new(AllRouteSelector::default()),
            result_selector: Box::new(BestResultSelector::default()),
            insertion_heuristic: Default::default(),
        }
    }
}

impl Default for RecreateWithGaps {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Recreate for RecreateWithGaps {
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
