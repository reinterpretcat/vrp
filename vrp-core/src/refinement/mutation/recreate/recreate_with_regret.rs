extern crate rayon;

use self::rayon::prelude::*;

use crate::construction::heuristics::*;
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::refinement::mutation::Recreate;
use crate::refinement::RefinementContext;
use std::cmp::Ordering::*;
use std::ops::Deref;

/// A recreate method which uses regret insertion approach.
pub struct RecreateWithRegret {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithRegret {
    fn default() -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(RegretJobMapReducer::default()),
        }
    }
}

impl Recreate for RecreateWithRegret {
    fn run(&self, _refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(&self.job_selector, &self.job_reducer, insertion_ctx)
    }
}

struct RegretJobMapReducer {
    regret_range: (i32, i32),
}

impl Default for RegretJobMapReducer {
    fn default() -> Self {
        Self { regret_range: (2, 4) }
    }
}

impl JobMapReducer for RegretJobMapReducer {
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        map: Box<dyn Fn(&Job) -> InsertionResult + Send + Sync + 'a>,
    ) -> InsertionResult {
        let mut results: Vec<InsertionResult> = jobs.par_iter().map(|job| map.deref()(&job)).collect();

        results.sort_by(|a, b| match (a, b) {
            (InsertionResult::Success(a), InsertionResult::Success(b)) => a.cost.partial_cmp(&b.cost).unwrap_or(Less),
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => Less,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => Greater,
            (InsertionResult::Failure(_), InsertionResult::Failure(_)) => Equal,
        });

        let regret_index =
            ctx.random.uniform_int(self.regret_range.0, self.regret_range.1).min(results.len() as i32) as usize - 1;

        let insertion_result = results
            .drain(regret_index..regret_index + 1)
            .next()
            .unwrap_or_else(|| panic!("Unexpected insertion results length"));

        insertion_result
    }
}
