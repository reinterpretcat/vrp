use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rand::prelude::*;
use rosomaxa::prelude::Random;
use std::sync::Arc;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
    max_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        // TODO we should prefer to always insert locked jobs
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        // TODO improve formula
        let max_jobs = self.min_jobs.max(ctx.solution.required.len().min(self.max_jobs));
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
    pub fn new(min_jobs: usize, max_jobs: usize, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(GapsJobSelector { min_jobs, max_jobs }),
                Box::new(AllRouteSelector::default()),
                Box::new(VariableLegSelector::new(random)),
                Box::new(BestResultSelector::default()),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithGaps {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
