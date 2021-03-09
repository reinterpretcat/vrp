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
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    insertion_heuristic: InsertionHeuristic,
}

impl Default for RecreateWithSkipBest {
    fn default() -> Self {
        RecreateWithSkipBest::new(1, 2)
    }
}

impl Recreate for RecreateWithSkipBest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.insertion_heuristic.process(
            insertion_ctx,
            self.job_selector.as_ref(),
            self.route_selector.as_ref(),
            self.result_selector.as_ref(),
            &refinement_ctx.quota,
        )
    }
}

impl RecreateWithSkipBest {
    /// Creates a new instance of `RecreateWithSkipBest`.
    pub fn new(min: usize, max: usize) -> Self {
        Self {
            job_selector: Box::new(AllJobSelector::default()),
            route_selector: Box::new(AllRouteSelector::default()),
            result_selector: Box::new(BestResultSelector::default()),
            insertion_heuristic: InsertionHeuristic::new(Box::new(SkipBestInsertionEvaluator::new(min, max))),
        }
    }
}

struct SkipBestInsertionEvaluator {
    min: usize,
    max: usize,
    fallback_evaluator: Box<dyn InsertionEvaluator + Send + Sync>,
}

impl SkipBestInsertionEvaluator {
    /// Creates a new instance of `SkipBestInsertionEvaluator`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min > 0);
        assert!(min <= max);

        Self { min, max, fallback_evaluator: Box::new(PositionInsertionEvaluator::default()) }
    }
}

impl InsertionEvaluator for SkipBestInsertionEvaluator {
    fn evaluate_one(
        &self,
        ctx: &InsertionContext,
        job: &Job,
        routes: &[RouteContext],
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_one(ctx, job, routes, result_selector)
    }

    fn evaluate_all(
        &self,
        ctx: &InsertionContext,
        jobs: &[Job],
        routes: &[RouteContext],
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        let skip_index = ctx.environment.random.uniform_int(self.min as i32, self.max as i32);

        // NOTE no need to proceed with skip, fallback to more performant reducer
        if skip_index == 1 || jobs.len() == 1 {
            return self.fallback_evaluator.evaluate_all(ctx, jobs, routes, result_selector);
        }

        let mut results = parallel_collect(&jobs, |job| self.evaluate_one(ctx, job, routes, result_selector));

        // TODO use result_selector?
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
