extern crate rand;

use crate::construction::heuristics::{InsertionHeuristic, JobSelector, ResultSelector};
use crate::construction::states::InsertionContext;
use crate::models::problem::Job;
use crate::refinement::recreate::{BestResultSelector, Recreate};
use crate::refinement::RefinementContext;
use rand::prelude::*;
use std::sync::Arc;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
        // TODO we should prefer to always insert locked jobs
        ctx.solution.required.shuffle(&mut rand::thread_rng());

        // TODO improve formula
        let max_jobs = self.min_jobs.max(ctx.solution.required.len());
        let take_jobs = ctx.random.uniform_int(self.min_jobs as i32, max_jobs as i32) as usize;

        Box::new(ctx.solution.required.iter().take(take_jobs).cloned())
    }
}

pub struct RecreateWithGaps {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl RecreateWithGaps {
    pub fn new(min_jobs: usize) -> Self {
        Self {
            job_selector: Box::new(GapsJobSelector { min_jobs }),
            result_selector: Box::new(BestResultSelector::default()),
        }
    }
}

impl Default for RecreateWithGaps {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Recreate for RecreateWithGaps {
    fn run(&self, _refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::process(&self.job_selector, &self.result_selector, insertion_ctx)
    }
}
