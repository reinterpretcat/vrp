#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/selectors_test.rs"]
mod selectors_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::models::solution::Leg;
use crate::utils::*;
use rand::prelude::*;
use rosomaxa::utils::{map_reduce, parallel_collect, Random};
use std::cmp::Ordering;
use std::sync::Arc;

/// On each insertion step, selects a list of routes where jobs can be inserted.
/// It is up to implementation to decide whether list consists of all possible routes or just some subset.
pub trait RouteSelector {
    /// This method is called before select. It allows to apply some changes on mutable context
    /// before immutable borrowing could happen within select method.
    /// Default implementation simply shuffles existing routes.
    fn prepare(&self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.routes.shuffle(&mut insertion_ctx.environment.random.get_rng());
    }

    /// Returns routes for job insertion.
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a InsertionContext,
        jobs: &[&'a Job],
    ) -> Box<dyn Iterator<Item = &'a RouteContext> + 'a>;
}

/// Returns a list of all possible routes for insertion.
#[derive(Default)]
pub struct AllRouteSelector {}

impl RouteSelector for AllRouteSelector {
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a InsertionContext,
        _: &[&'a Job],
    ) -> Box<dyn Iterator<Item = &'a RouteContext> + 'a> {
        Box::new(insertion_ctx.solution.routes.iter().chain(insertion_ctx.solution.registry.next_route()))
    }
}

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether list consists of all jobs or just some subset.
pub trait JobSelector {
    /// This method is called before select. It allows to apply some changes on mutable context
    /// before immutable borrowing could happen within select method.
    /// Default implementation simply shuffles jobs in required collection.
    fn prepare(&self, insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.required.shuffle(&mut insertion_ctx.environment.random.get_rng());
    }

    /// Returns a portion of all jobs.
    fn select<'a>(&'a self, insertion_ctx: &'a InsertionContext) -> Box<dyn Iterator<Item = &'a Job> + 'a> {
        Box::new(insertion_ctx.solution.required.iter())
    }
}

/// Returns a list of all jobs to be inserted.
#[derive(Default)]
pub struct AllJobSelector {}

impl JobSelector for AllJobSelector {}

/// Evaluates insertion.
pub trait InsertionEvaluator {
    /// Evaluates insertion of a single job into given collection of routes.
    fn evaluate_job(
        &self,
        insertion_ctx: &InsertionContext,
        job: &Job,
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult;

    /// Evaluates insertion of multiple jobs into given route.
    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[&Job],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult;

    /// Evaluates insertion of a job collection into given collection of routes.
    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
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
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
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
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
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
        jobs: &[&Job],
        leg_selection: &LegSelection,
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
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
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
    fn select_cost<'a>(
        &self,
        left: &'a InsertionCost,
        right: &'a InsertionCost,
    ) -> Either<&'a InsertionCost, &'a InsertionCost> {
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
                let left_cost: InsertionCost = self.noise.generate_multi(left_success.cost.iter()).collect();
                let right_cost = self.noise.generate_multi(right_success.cost.iter()).collect();

                match left_cost.cmp(&right_cost) {
                    Ordering::Less => left,
                    Ordering::Greater => right,
                    Ordering::Equal if self.noise.random().is_head_not_tails() => left,
                    _ => right,
                }
            }
            _ => right,
        }
    }

    fn select_cost<'a>(
        &self,
        left: &'a InsertionCost,
        right: &'a InsertionCost,
    ) -> Either<&'a InsertionCost, &'a InsertionCost> {
        let left_cost: InsertionCost = self.noise.generate_multi(left.iter()).collect();
        let right_cost: InsertionCost = self.noise.generate_multi(right.iter()).collect();

        match left_cost.cmp(&right_cost) {
            Ordering::Less => Either::Left(left),
            Ordering::Greater => Either::Right(right),
            Ordering::Equal if self.noise.random().is_head_not_tails() => Either::Left(left),
            _ => Either::Right(right),
        }
    }
}

/// Selects a job with the highest cost insertion occurs into a new route.
#[derive(Default)]
pub struct FarthestResultSelector {}

impl ResultSelector for FarthestResultSelector {
    fn select_insertion(
        &self,
        insertion_ctx: &InsertionContext,
        left: InsertionResult,
        right: InsertionResult,
    ) -> InsertionResult {
        match (&left, &right) {
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => left,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => right,
            (InsertionResult::Success(lhs), InsertionResult::Success(rhs)) => {
                let routes = &insertion_ctx.solution.routes;
                let lhs_route = routes.iter().find(|route_ctx| route_ctx.route().actor == lhs.actor);
                let rhs_route = routes.iter().find(|route_ctx| route_ctx.route().actor == rhs.actor);

                let insert_right = match (lhs_route.is_some(), rhs_route.is_some()) {
                    (false, false) => lhs.cost < rhs.cost,
                    (true, false) => false,
                    (false, true) => true,
                    (true, true) => lhs.cost > rhs.cost,
                };

                if insert_right {
                    right
                } else {
                    left
                }
            }
            _ => right,
        }
    }
}

/// A result selector strategy inspired by "Slack Induction by String Removals for Vehicle
/// Routing Problems", Jan Christiaens, Greet Vanden Berghe.
pub struct BlinkResultSelector {
    random: Arc<dyn Random + Send + Sync>,
    ratio: f64,
}

impl BlinkResultSelector {
    /// Creates an instance of `BlinkResultSelector`.
    pub fn new(ratio: f64, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { random, ratio }
    }

    /// Creates an instance of `BlinkResultSelector` with default values.
    pub fn new_with_defaults(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self::new(0.01, random)
    }
}

impl ResultSelector for BlinkResultSelector {
    fn select_insertion(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        let is_blink = self.random.is_hit(self.ratio);

        if is_blink {
            return if self.random.is_head_not_tails() { left } else { right };
        }

        InsertionResult::choose_best_result(left, right)
    }

    fn select_cost<'a>(
        &self,
        left: &'a InsertionCost,
        right: &'a InsertionCost,
    ) -> Either<&'a InsertionCost, &'a InsertionCost> {
        let is_blink = self.random.is_hit(self.ratio);

        if is_blink {
            return if self.random.is_head_not_tails() { Either::Left(left) } else { Either::Right(right) };
        }

        match left.cmp(right) {
            Ordering::Less => Either::Left(left),
            Ordering::Greater => Either::Right(right),
            Ordering::Equal if self.random.is_head_not_tails() => Either::Left(left),
            _ => Either::Right(right),
        }
    }
}

/// Keeps either specific result selector implementation or multiple implementations.
pub enum ResultSelection {
    /// Returns a provider which returns one of built-in result selectors non-deterministically.
    Stochastic(ResultSelectorProvider),

    /// Returns concrete instance of result selector to be used.
    Concrete(Box<dyn ResultSelector + Send + Sync>),
}

/// Provides way to access one of built-in result selectors non-deterministically.
pub struct ResultSelectorProvider {
    inners: Vec<Box<dyn ResultSelector + Send + Sync>>,
    weights: Vec<usize>,
    random: Arc<dyn Random + Send + Sync>,
}

impl ResultSelectorProvider {
    /// Creates a new instance of `StochasticResultSelectorFn`
    pub fn new_default(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            inners: vec![
                Box::<BestResultSelector>::default(),
                Box::new(NoiseResultSelector::new(Noise::new_with_addition(0.05, (-0.25, 0.25), random.clone()))),
                Box::new(BlinkResultSelector::new_with_defaults(random.clone())),
                Box::<FarthestResultSelector>::default(),
            ],
            weights: vec![60, 10, 10, 20],
            random,
        }
    }

    /// Returns random result selector from the list.
    pub fn pick(&self) -> &(dyn ResultSelector + Send + Sync) {
        self.inners[self.random.weighted(self.weights.as_slice())].as_ref()
    }
}

/// Provides way to control routing leg selection mode.
#[derive(Clone)]
pub enum LegSelection {
    /// Stochastic mode: depending on route size, not all legs could be selected.
    Stochastic(Arc<dyn Random + Send + Sync>),
    /// Exhaustive mode: all legs are selected.
    Exhaustive,
}

impl LegSelection {
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
                .route()
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
            unwrap_from_result(route_ctx.route().tour.legs().skip(skip).try_fold(init, |acc, leg| map_fn(leg, acc)))
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

                let total_legs = route_ctx.route().tour.legs().size_hint().0;
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
