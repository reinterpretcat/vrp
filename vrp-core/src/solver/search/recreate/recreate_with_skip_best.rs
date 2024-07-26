use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::search::{ConfigurableRecreate, Recreate};
use crate::solver::RefinementContext;
use rosomaxa::prelude::Random;
use std::cmp::Ordering::*;
use std::sync::Arc;

/// A recreate strategy which skips best job insertion for insertion.
pub struct RecreateWithSkipBest {
    recreate: ConfigurableRecreate,
}

impl Recreate for RecreateWithSkipBest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}

impl RecreateWithSkipBest {
    /// Creates a new instance of `RecreateWithSkipBest`.
    pub fn new(min: usize, max: usize, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<AllJobSelector>::default(),
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random.clone()),
                ResultSelection::Stochastic(ResultSelectorProvider::new_default(random)),
                InsertionHeuristic::new(Box::new(SkipBestInsertionEvaluator::new(min, max))),
            ),
        }
    }
}

struct SkipBestInsertionEvaluator {
    min: usize,
    max: usize,
    fallback_evaluator: PositionInsertionEvaluator,
}

impl SkipBestInsertionEvaluator {
    /// Creates a new instance of `SkipBestInsertionEvaluator`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min > 0);
        assert!(min <= max);

        Self { min, max, fallback_evaluator: PositionInsertionEvaluator::default() }
    }
}

impl InsertionEvaluator for SkipBestInsertionEvaluator {
    fn evaluate_job(
        &self,
        insertion_ctx: &InsertionContext,
        job: &Job,
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_job(insertion_ctx, job, routes, leg_selection, result_selector)
    }

    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[&Job],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_route(insertion_ctx, route_ctx, jobs, leg_selection, result_selector)
    }

    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        let skip_index = insertion_ctx.environment.random.uniform_int(self.min as i32, self.max as i32);

        // NOTE no need to proceed with skip, fallback to more performant reducer
        if skip_index == 1 || jobs.len() == 1 || routes.is_empty() {
            return self.fallback_evaluator.evaluate_all(insertion_ctx, jobs, routes, leg_selection, result_selector);
        }

        let mut results = self.fallback_evaluator.evaluate_and_collect_all(
            insertion_ctx,
            jobs,
            routes,
            leg_selection,
            result_selector,
        );

        // TODO use result_selector?
        results.sort_by(|a, b| match (a, b) {
            (InsertionResult::Success(a), InsertionResult::Success(b)) => a.cost.partial_cmp(&b.cost).unwrap_or(Less),
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => Less,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => Greater,
            (
                InsertionResult::Failure(InsertionFailure { constraint: left, .. }),
                InsertionResult::Failure(InsertionFailure { constraint: right, .. }),
            ) => match (left.is_unknown(), right.is_unknown()) {
                (true, _) => Greater,
                (_, true) => Less,
                _ => Equal,
            },
        });

        let skip_index = skip_index.min(results.len() as i32) as usize - 1;

        let insertion_result = results
            .drain(skip_index..=skip_index)
            .next()
            .unwrap_or_else(|| panic!("Unexpected insertion results length"));

        insertion_result
    }
}
