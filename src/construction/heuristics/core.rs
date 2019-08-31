use crate::construction::states::{InsertionContext, InsertionResult};
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
        //        ctx.solution.routes.iter().chain(ctx.solution.registry.next())
        //            .fold(InsertionResult::make_failure())
        InsertionResult::make_failure()
    }
}
