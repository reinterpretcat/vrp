use crate::construction::heuristics::{InsertionHeuristic, JobSelector};
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

pub struct RecreateWithCheapest {}

impl RecreateWithCheapest {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RecreateWithCheapest {
    fn default() -> Self {
        Self::new()
    }
}

impl Recreate for RecreateWithCheapest {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::new(Box::new(AllJobSelector {}), Box::new(BestResultSelector {})).process(insertion_ctx)
    }
}
