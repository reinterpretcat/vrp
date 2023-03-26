use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rosomaxa::prelude::Random;
use std::sync::Arc;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
    max_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, insertion_ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        // TODO improve formula
        let max_jobs = self.min_jobs.max(insertion_ctx.solution.required.len().min(self.max_jobs));
        let take_jobs = insertion_ctx.environment.random.uniform_int(self.min_jobs as i32, max_jobs as i32) as usize;

        Box::new(insertion_ctx.solution.required.iter().take(take_jobs))
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
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random),
                Box::<BestResultSelector>::default(),
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
