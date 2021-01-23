#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/selectors_test.rs"]
mod selectors_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::utils::{map_reduce, Noise};
use rand::prelude::*;

/// On each insertion step, selects a list of routes where jobs can be inserted.
/// It is up to implementation to decide whether list consists of all possible routes or just some subset.
pub trait RouteSelector {
    /// Returns routes for job insertion.
    fn select<'a>(&'a self, ctx: &'a InsertionContext, job: &'a Job) -> Box<dyn Iterator<Item = RouteContext> + 'a>;
}

/// Returns a list of all possible routes for insertion.
pub struct AllRouteSelector {}

impl Default for AllRouteSelector {
    fn default() -> Self {
        Self {}
    }
}

impl RouteSelector for AllRouteSelector {
    fn select<'a>(&'a self, ctx: &'a InsertionContext, _job: &'a Job) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        Box::new(ctx.solution.routes.iter().cloned().chain(ctx.solution.registry.next()))
    }
}

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether list consists of all jobs or just some subset.
pub trait JobSelector {
    /// Returns a portion of all jobs.
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a>;
}

/// Returns a list of all jobs to be inserted.
pub struct AllJobSelector {}

impl Default for AllJobSelector {
    fn default() -> Self {
        Self {}
    }
}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        Box::new(ctx.solution.required.iter().cloned())
    }
}

/// A job collection reducer.
pub trait JobMapReducer {
    /// Reduces job collection into single insertion result
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult;
}

/// A job map reducer which compares pairs of insertion results and pick one from those.
pub struct PairJobMapReducer {
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl PairJobMapReducer {
    /// Creates a new instance of `PairJobMapReducer`.
    pub fn new(
        route_selector: Box<dyn RouteSelector + Send + Sync>,
        result_selector: Box<dyn ResultSelector + Send + Sync>,
    ) -> Self {
        Self { route_selector, result_selector }
    }
}

impl JobMapReducer for PairJobMapReducer {
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        insertion_position: InsertionPosition,
    ) -> InsertionResult {
        map_reduce(
            &jobs,
            |job| {
                evaluate_job_insertion(
                    &job,
                    &ctx,
                    self.route_selector.as_ref(),
                    self.result_selector.as_ref(),
                    insertion_position,
                )
            },
            InsertionResult::make_failure,
            |a, b| self.result_selector.select(&ctx, a, b),
        )
    }
}

/// Insertion result selector.
pub trait ResultSelector {
    /// Selects one insertion result from two to promote as best.
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult;
}

/// Selects best result.
pub struct BestResultSelector {}

impl Default for BestResultSelector {
    fn default() -> Self {
        Self {}
    }
}

impl ResultSelector for BestResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

/// Selects results with noise.
pub struct NoiseResultSelector {
    noise: Noise,
}

impl NoiseResultSelector {
    /// Creates a new instance of `NoiseResultSelector`.
    pub fn new(noise: Noise) -> Self {
        Self { noise }
    }
}

impl ResultSelector for NoiseResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        match (&left, &right) {
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => left,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => right,
            (InsertionResult::Success(left_success), InsertionResult::Success(right_success)) => {
                let left_cost = self.noise.add(left_success.cost);
                let right_cost = self.noise.add(right_success.cost);

                if left_cost < right_cost {
                    left
                } else {
                    right
                }
            }
            _ => right,
        }
    }
}
