use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::mutation::Recreate;
use crate::solver::RefinementContext;
use crate::utils::parallel_collect;
use std::cmp::Ordering::*;

/// A recreate strategy which skips best job insertion for insertion.
pub struct RecreateWithSkipBest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithSkipBest {
    fn default() -> Self {
        RecreateWithSkipBest::new(1, 2)
    }
}

impl Recreate for RecreateWithSkipBest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}

impl RecreateWithSkipBest {
    /// Creates a new instance of `RecreateWithSkipBest`.
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(SkipBestJobMapReducer::new(min, max)),
        }
    }
}

struct SkipBestJobMapReducer {
    min: usize,
    max: usize,
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    inner_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl SkipBestJobMapReducer {
    /// Creates a new instance of `SkipBestJobMapReducer`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min > 0);
        assert!(min <= max);

        Self {
            min,
            max,
            route_selector: Box::new(AllRouteSelector::default()),
            result_selector: Box::new(BestResultSelector::default()),
            inner_reducer: Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BestResultSelector::default()),
            )),
        }
    }
}

impl JobMapReducer for SkipBestJobMapReducer {
    #[allow(clippy::let_and_return)]
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult {
        let skip_index = ctx.environment.random.uniform_int(self.min as i32, self.max as i32);

        // NOTE no need to proceed with skip, fallback to more performant reducer
        if skip_index == 1 || jobs.len() == 1 {
            return self.inner_reducer.reduce(ctx, jobs, insertion_position);
        }

        let mut results = parallel_collect(&jobs, ctx.environment.parallelism.inner_degree.clone(), |job| {
            evaluate_job_insertion(
                &job,
                &ctx,
                self.route_selector.as_ref(),
                self.result_selector.as_ref(),
                insertion_position,
            )
        });

        results.sort_by(|a, b| match (a, b) {
            (InsertionResult::Success(a), InsertionResult::Success(b)) => a.cost.partial_cmp(&b.cost).unwrap_or(Less),
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => Less,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => Greater,
            (InsertionResult::Failure(_), InsertionResult::Failure(_)) => Equal,
        });

        let skip_index = skip_index.min(results.len() as i32) as usize - 1;

        let insertion_result = results
            .drain(skip_index..=skip_index)
            .next()
            .unwrap_or_else(|| panic!("Unexpected insertion results length"));

        insertion_result
    }
}
