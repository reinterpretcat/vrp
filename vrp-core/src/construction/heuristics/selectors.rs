#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/selectors_test.rs"]
mod selectors_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::models::solution::Leg;
use crate::utils::*;
use rand::prelude::*;
use rosomaxa::utils::{map_reduce, parallel_collect, Random};
use std::sync::Arc;

/// On each insertion step, selects a list of routes where jobs can be inserted.
/// It is up to implementation to decide whether list consists of all possible routes or just some subset.
pub trait RouteSelector {
    /// Returns routes for job insertion.
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a mut InsertionContext,
        jobs: &[Job],
    ) -> Box<dyn Iterator<Item = RouteContext> + 'a>;
}

/// Returns a list of all possible routes for insertion.
#[derive(Default)]
pub struct AllRouteSelector {}

impl RouteSelector for AllRouteSelector {
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a mut InsertionContext,
        _jobs: &[Job],
    ) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        insertion_ctx.solution.routes.shuffle(&mut insertion_ctx.environment.random.get_rng());
        Box::new(insertion_ctx.solution.routes.iter().cloned().chain(insertion_ctx.solution.registry.next()))
    }
}

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether list consists of all jobs or just some subset.
pub trait JobSelector {
    /// Returns a portion of all jobs.
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a>;
}

/// Returns a list of all jobs to be inserted.
#[derive(Default)]
pub struct AllJobSelector {}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, insertion_ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        insertion_ctx.solution.required.shuffle(&mut insertion_ctx.environment.random.get_rng());

        Box::new(insertion_ctx.solution.required.iter().cloned())
    }
}

/// Evaluates insertion.
pub trait InsertionEvaluator {
    /// Evaluates insertion of a single job into given collection of routes.
    fn evaluate_job(
        &self,
        insertion_ctx: &InsertionContext,
        job: &Job,
        routes: &[RouteContext],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult;

    /// Evaluates insertion of multiple jobs into given route.
    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[Job],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult;

    /// Evaluates insertion of a job collection into given collection of routes.
    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[Job],
        routes: &[RouteContext],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult;
}

/// Evaluates job insertion in routes at given position.
pub struct PositionInsertionEvaluator {
    insertion_position: InsertionPosition,
}

impl Default for PositionInsertionEvaluator {
    fn default() -> Self {
        Self::new(InsertionPosition::Any)
    }
}

impl PositionInsertionEvaluator {
    /// Creates a new instance of `PositionInsertionEvaluator`.
    pub fn new(insertion_position: InsertionPosition) -> Self {
        Self { insertion_position }
    }

    /// Evaluates all jobs ad routes.
    pub(crate) fn evaluate_and_collect_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[Job],
        routes: &[RouteContext],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> Vec<InsertionResult> {
        if Self::is_fold_jobs(insertion_ctx) {
            parallel_collect(jobs, |job| self.evaluate_job(insertion_ctx, job, routes, leg_selection, result_selector))
        } else {
            parallel_collect(routes, |route_ctx| {
                self.evaluate_route(insertion_ctx, route_ctx, jobs, leg_selection, result_selector)
            })
        }
    }

    fn is_fold_jobs(insertion_ctx: &InsertionContext) -> bool {
        insertion_ctx.solution.required.len() > insertion_ctx.solution.routes.len()
    }
}

impl InsertionEvaluator for PositionInsertionEvaluator {
    fn evaluate_job(
        &self,
        insertion_ctx: &InsertionContext,
        job: &Job,
        routes: &[RouteContext],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        let eval_ctx = EvaluationContext { goal: &insertion_ctx.problem.goal, job, leg_selection, result_selector };

        routes.iter().fold(InsertionResult::make_failure(), |acc, route_ctx| {
            eval_job_insertion_in_route(insertion_ctx, &eval_ctx, route_ctx, self.insertion_position, acc)
        })
    }

    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[Job],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        jobs.iter().fold(InsertionResult::make_failure(), |acc, job| {
            let eval_ctx = EvaluationContext { goal: &insertion_ctx.problem.goal, job, leg_selection, result_selector };
            eval_job_insertion_in_route(insertion_ctx, &eval_ctx, route_ctx, self.insertion_position, acc)
        })
    }

    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[Job],
        routes: &[RouteContext],
        leg_selection: &LegSelectionMode,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        if Self::is_fold_jobs(insertion_ctx) {
            map_reduce(
                jobs,
                |job| self.evaluate_job(insertion_ctx, job, routes, leg_selection, result_selector),
                InsertionResult::make_failure,
                |a, b| result_selector.select_insertion(insertion_ctx, a, b),
            )
        } else {
            map_reduce(
                routes,
                |route| self.evaluate_route(insertion_ctx, route, jobs, leg_selection, result_selector),
                InsertionResult::make_failure,
                |a, b| result_selector.select_insertion(insertion_ctx, a, b),
            )
        }
    }
}

/// Insertion result selector.
pub trait ResultSelector {
    /// Selects one insertion result from two to promote as best.
    fn select_insertion(
        &self,
        ctx: &InsertionContext,
        left: InsertionResult,
        right: InsertionResult,
    ) -> InsertionResult;

    /// Selects one insertion result from two to promote as best.
    fn select_cost(&self, _route_ctx: &RouteContext, left: f64, right: f64) -> Either<f64, f64> {
        if left < right {
            Either::Left(left)
        } else {
            Either::Right(right)
        }
    }
}

/// Selects best result.
#[derive(Default)]
pub struct BestResultSelector {}

impl ResultSelector for BestResultSelector {
    fn select_insertion(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
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
    fn select_insertion(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        match (&left, &right) {
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => left,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => right,
            (InsertionResult::Success(left_success), InsertionResult::Success(right_success)) => {
                let left_cost = left_success.cost + self.noise.generate(left_success.cost);
                let right_cost = right_success.cost + self.noise.generate(right_success.cost);

                if left_cost < right_cost {
                    left
                } else {
                    right
                }
            }
            _ => right,
        }
    }

    fn select_cost(&self, _route_ctx: &RouteContext, left: f64, right: f64) -> Either<f64, f64> {
        let left = left + self.noise.generate(left);
        let right = right + self.noise.generate(right);

        if left < right {
            Either::Left(left)
        } else {
            Either::Right(right)
        }
    }
}

/// Provides way to control routing leg selection mode.
#[derive(Clone)]
pub enum LegSelectionMode {
    /// Stochastic mode: depending on route size, not all legs could be selected.
    Stochastic(Arc<dyn Random + Send + Sync>),
    /// Exhaustive mode: all legs are selected.
    Exhaustive,
}

impl LegSelectionMode {
    /// Selects a best leg for insertion.
    pub(crate) fn sample_best<R, FM, FC>(
        &self,
        route_ctx: &RouteContext,
        job: &Job,
        skip: usize,
        init: R,
        mut map_fn: FM,
        compare_fn: FC,
    ) -> R
    where
        R: Default,
        FM: FnMut(Leg, R) -> Result<R, R>,
        FC: Fn(&R, &R) -> bool,
    {
        if let Some((sample_size, random)) = self.get_sample_data(route_ctx, job, skip) {
            route_ctx
                .route
                .tour
                .legs()
                .skip(skip)
                .sample_search(
                    sample_size,
                    random.clone(),
                    &mut |leg: Leg<'_>| unwrap_from_result(map_fn(leg, R::default())),
                    |leg: &Leg<'_>| leg.1 as i32,
                    &compare_fn,
                )
                .unwrap_or(init)
        } else {
            unwrap_from_result(route_ctx.route.tour.legs().skip(skip).try_fold(init, |acc, leg| map_fn(leg, acc)))
        }
    }

    /// Returns a sample data for stochastic mode.
    fn get_sample_data(
        &self,
        route_ctx: &RouteContext,
        job: &Job,
        skip: usize,
    ) -> Option<(usize, Arc<dyn Random + Send + Sync>)> {
        match self {
            Self::Stochastic(random) => {
                let gen_usize = |min: i32, max: i32| random.uniform_int(min, max) as usize;
                let greedy_threshold = match job {
                    Job::Single(_) => gen_usize(12, 24),
                    Job::Multi(_) => gen_usize(8, 16),
                };

                let total_legs = route_ctx.route.tour.legs().size_hint().0;
                let visit_legs = if total_legs > skip { total_legs - skip } else { 0 };

                if visit_legs < greedy_threshold {
                    None
                } else {
                    Some((
                        match job {
                            Job::Single(_) => 8,
                            Job::Multi(_) => 4,
                        },
                        random.clone(),
                    ))
                }
            }
            Self::Exhaustive => None,
        }
    }
}
