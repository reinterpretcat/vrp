extern crate rayon;

use self::rayon::slice::Iter;
use crate::construction::heuristics::{InsertionHeuristic, JobSelector, ResultSelector};
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::refinement::recreate::Recreate;
use rayon::prelude::*;
use std::sync::Arc;

/// Returns a list of all jobs to be inserted.
struct AllJobSelector {}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a InsertionContext) -> Iter<Arc<Job>> {
        ctx.solution.required.par_iter()
    }
}

/// Selects best result.
struct BestResultSelector {}

impl ResultSelector for BestResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
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
