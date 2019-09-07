use crate::construction::states::{
    InsertionContext, InsertionProgress, InsertionResult, RouteContext,
};
use crate::models::problem::{Job, Multi, Single};
use std::borrow::Borrow;
use std::sync::Arc;

/// Provides the way to evaluate insertion cost.
pub struct InsertionEvaluator {}

impl InsertionEvaluator {
    pub fn new() -> Self {
        InsertionEvaluator {}
    }

    /// Evaluates possibility to preform insertion from given insertion context.
    pub fn evaluate(&self, job: &Arc<Job>, ctx: &InsertionContext) -> InsertionResult {
        ctx.solution
            .routes
            .iter()
            .cloned()
            .chain(ctx.solution.registry.next().map(|a| RouteContext::new(a)))
            .fold(InsertionResult::make_failure(), |acc, route_ctx| {
                if let Some(violation) = ctx.problem.constraint.evaluate_hard_route(&route_ctx, job)
                {
                    return InsertionResult::choose_best_result(
                        acc,
                        InsertionResult::make_failure_with_code(violation.code),
                    );
                }

                let progress = InsertionProgress {
                    cost: match acc.borrow() {
                        InsertionResult::Success(success) => success.cost,
                        _ => std::f64::MAX,
                    },
                    completeness: ctx.progress.completeness,
                    total: ctx.progress.total,
                };

                InsertionResult::choose_best_result(
                    acc,
                    match job.borrow() {
                        Job::Single(single) => {
                            Self::evaluate_single(job, single, ctx, &route_ctx, &progress)
                        }
                        Job::Multi(multi) => {
                            Self::evaluate_multi(job, multi, ctx, &route_ctx, &progress)
                        }
                    },
                )
            })
    }

    fn evaluate_single(
        job: &Arc<Job>,
        single: &Single,
        ctx: &InsertionContext,
        route_context: &RouteContext,
        progress: &InsertionProgress,
    ) -> InsertionResult {
        unimplemented!()
    }

    fn evaluate_multi(
        job: &Arc<Job>,
        multi: &Multi,
        ctx: &InsertionContext,
        route_context: &RouteContext,
        progress: &InsertionProgress,
    ) -> InsertionResult {
        unimplemented!()
    }
}
