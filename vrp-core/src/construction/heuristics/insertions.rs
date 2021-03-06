use crate::construction::heuristics::*;
use crate::construction::Quota;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::solution::Activity;
use std::sync::Arc;

/// Specifies insertion result variant.
pub enum InsertionResult {
    /// Successful insertion result.
    Success(InsertionSuccess),
    /// Insertion failure.
    Failure(InsertionFailure),
}

/// Specifies insertion success result needed to insert job into tour.
pub struct InsertionSuccess {
    /// Specifies delta cost change for the insertion.
    pub cost: Cost,

    /// Original job to be inserted.
    pub job: Job,

    /// Specifies activities within index where they have to be inserted.
    pub activities: Vec<(Activity, usize)>,

    /// Specifies route context where insertion happens.
    pub context: RouteContext,
}

/// Specifies insertion failure.
pub struct InsertionFailure {
    /// Failed constraint code.
    pub constraint: i32,
    /// A flag which signalizes that algorithm should stop trying to insert at next positions.
    pub stopped: bool,
    /// Original job failed to be inserted.
    pub job: Option<Job>,
}

/// Implements generalized insertion heuristic.
/// Using `JobSelector`, `RouteSelector`, and `ResultSelector` it tries to identify next job to
/// be inserted until there are no jobs left or it is not possible to insert due to constraint
/// limitations.
pub struct InsertionHeuristic {
    insertion_evaluator: Box<dyn InsertionEvaluator + Send + Sync>,
}

impl Default for InsertionHeuristic {
    fn default() -> Self {
        InsertionHeuristic::new(Box::new(PositionInsertionEvaluator::default()))
    }
}

impl InsertionHeuristic {
    /// Creates a new instance of `InsertionHeuristic`.
    pub fn new(insertion_evaluator: Box<dyn InsertionEvaluator + Send + Sync>) -> Self {
        Self { insertion_evaluator }
    }
}

impl InsertionHeuristic {
    /// Runs common insertion heuristic algorithm using given selector specializations.
    pub fn process(
        &self,
        ctx: InsertionContext,
        job_selector: &(dyn JobSelector + Send + Sync),
        route_selector: &(dyn RouteSelector + Send + Sync),
        result_selector: &(dyn ResultSelector + Send + Sync),
        quota: &Option<Arc<dyn Quota + Send + Sync>>,
    ) -> InsertionContext {
        let mut ctx = ctx;

        prepare_insertion_ctx(&mut ctx);

        while !ctx.solution.required.is_empty() && !quota.as_ref().map_or(false, |q| q.is_reached()) {
            let jobs = job_selector.select(&mut ctx).collect::<Vec<Job>>();
            let routes = route_selector.select(&mut ctx, jobs.as_slice()).collect::<Vec<RouteContext>>();

            let result =
                self.insertion_evaluator.evaluate_all(&ctx, jobs.as_slice(), routes.as_slice(), result_selector);

            apply_insertion_result(&mut ctx, result);
        }

        finalize_insertion_ctx(&mut ctx);

        ctx
    }
}

impl InsertionResult {
    /// Creates result which represents insertion success.
    pub fn make_success(cost: Cost, job: Job, activities: Vec<(Activity, usize)>, route_ctx: RouteContext) -> Self {
        Self::Success(InsertionSuccess { cost, job, activities, context: route_ctx })
    }

    /// Creates result which represents insertion failure.
    pub fn make_failure() -> Self {
        Self::make_failure_with_code(-1, false, None)
    }

    /// Creates result which represents insertion failure with given code.
    pub fn make_failure_with_code(code: i32, stopped: bool, job: Option<Job>) -> Self {
        Self::Failure(InsertionFailure { constraint: code, stopped, job })
    }

    /// Compares two insertion results and returns the cheapest by cost.
    pub fn choose_best_result(left: Self, right: Self) -> Self {
        match (&left, &right) {
            (Self::Success(_), Self::Failure(_)) => left,
            (Self::Failure(_), Self::Success(_)) => right,
            (Self::Success(lhs), Self::Success(rhs)) => {
                if lhs.cost > rhs.cost {
                    right
                } else {
                    left
                }
            }
            _ => right,
        }
    }
}

pub(crate) fn prepare_insertion_ctx(ctx: &mut InsertionContext) {
    ctx.solution.required.extend(ctx.solution.unassigned.iter().map(|(job, _)| job.clone()));
    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

pub(crate) fn finalize_insertion_ctx(ctx: &mut InsertionContext) {
    finalize_unassigned(ctx, -1);

    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

pub(crate) fn apply_insertion_result(ctx: &mut InsertionContext, result: InsertionResult) {
    match result {
        InsertionResult::Success(success) => {
            let is_new_route = ctx.solution.registry.use_route(&success.context);
            let route_index = ctx.solution.routes.iter().position(|ctx| ctx == &success.context).unwrap_or_else(|| {
                assert!(is_new_route);
                ctx.solution.routes.push(success.context.deep_copy());
                ctx.solution.routes.len() - 1
            });

            let route_ctx = ctx.solution.routes.get_mut(route_index).unwrap();
            let route = route_ctx.route_mut();
            success.activities.into_iter().for_each(|(a, index)| {
                route.tour.insert_at(a, index + 1);
            });

            let job = success.job;
            ctx.solution.required.retain(|j| *j != job);
            ctx.solution.unassigned.remove(&job);
            ctx.problem.constraint.accept_insertion(&mut ctx.solution, route_index, &job);
        }
        InsertionResult::Failure(failure) => {
            if let Some(job) = failure.job {
                ctx.solution.unassigned.insert(job.clone(), failure.constraint);
                ctx.solution.required.retain(|j| *j != job);
            } else {
                // NOTE this happens when evaluator fails to insert jobs due to lack of routes in registry
                finalize_unassigned(ctx, failure.constraint)
            }
        }
    }
}

fn finalize_unassigned(ctx: &mut InsertionContext, code: i32) {
    let unassigned = &ctx.solution.unassigned;
    ctx.solution.required.retain(|job| !unassigned.contains_key(job));
    ctx.solution.unassigned.extend(ctx.solution.required.drain(0..).map(|job| (job, code)));
}
