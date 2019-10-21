extern crate rand;

use crate::construction::heuristics::{InsertionHeuristic, JobSelector};
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::refinement::recreate::{BestResultSelector, Recreate};
use rand::prelude::*;
use std::slice::Iter;
use std::sync::Arc;

/// Returns a sub set of randomly selected jobs.
struct GapsJobSelector {
    min_jobs: usize,
}

impl JobSelector for GapsJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
        // TODO move to Random struct?
        ctx.solution.required.shuffle(&mut rand::thread_rng());

        // TODO improve formula
        let max_jobs = self.min_jobs.max(ctx.solution.required.len());
        let take_jobs = ctx.random.uniform_int(self.min_jobs as i32, max_jobs as i32) as usize;

        Box::new(ctx.solution.required.iter().take(take_jobs).cloned())
    }
}

pub struct RecreateWithGaps {
    min_jobs: usize,
}

impl RecreateWithGaps {
    pub fn new(min_jobs: usize) -> Self {
        Self { min_jobs }
    }
}

impl Default for RecreateWithGaps {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Recreate for RecreateWithGaps {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::new(Box::new(GapsJobSelector { min_jobs: self.min_jobs }), Box::new(BestResultSelector {}))
            .process(insertion_ctx)
    }
}
