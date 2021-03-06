use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::mutation::ConfigurableRecreate;
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
    recreate: ConfigurableRecreate,
}

impl RecreateWithGaps {
    /// Creates a new instance of `RecreateWithGaps`.
    pub fn new(min_jobs: usize) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(GapsJobSelector { min_jobs }),
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
                Default::default(),
            ),
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
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
