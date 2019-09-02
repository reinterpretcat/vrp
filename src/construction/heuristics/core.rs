use crate::construction::states::{InsertionContext, InsertionResult, RouteContext};
use crate::models::problem::Job;
use std::sync::Arc;

/// Provides the way to evaluate insertion cost.
pub struct InsertionEvaluator {}

impl InsertionEvaluator {
    pub fn new() -> InsertionEvaluator {
        InsertionEvaluator {}
    }

    /// Evaluates possibility to preform insertion from given insertion context.
    pub fn evaluate(&self, job: &Arc<Job>, ctx: &InsertionContext) -> InsertionResult {
        ctx.solution
            .routes
            .iter()
            .cloned()
            .chain(ctx.solution.registry.next().map(|a| RouteContext::new(a)))
            .fold(InsertionResult::make_failure(), |acc, rCtx| acc)
    }
}
