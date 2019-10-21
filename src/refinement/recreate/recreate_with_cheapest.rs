use crate::construction::heuristics::{InsertionHeuristic, JobSelector, ResultSelector};
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::refinement::recreate::{BestResultSelector, Recreate};
use std::slice::Iter;
use std::sync::Arc;

/// Returns a list of all jobs to be inserted.
struct AllJobSelector {}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
        Box::new(ctx.solution.required.iter().cloned())
    }
}

pub struct RecreateWithCheapest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self { job_selector: Box::new(AllJobSelector {}), result_selector: Box::new(BestResultSelector::default()) }
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::process(&self.job_selector, &self.result_selector, insertion_ctx)
    }
}
