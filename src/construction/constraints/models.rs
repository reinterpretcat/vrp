use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::Job;
use std::sync::Arc;
use crate::models::Solution;

/// Specifies a base constraint behavior.
pub trait Constraint {
    /// Accept route and updates its state to allow more efficient constraint checks.
    /// Called in thread-safe context, so it is a chance to apply some changes.
    fn accept_route(ctx: &RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// Called in thread-safe context.
    fn accept_solution(ctx: &SolutionContext);

    /// Returns unique constraint state keys.
    fn state_keys() -> &'static Vec<i32>;
}

/// Specifies hard constraint which operates on route level.
pub trait HardRouteConstraint: Constraint {
    fn check_job(ctx: &RouteContext, job: &Arc<Job>) -> RouteCheckResult;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftRouteConstraint: Constraint {
    fn estimate_job(ctx: &RouteContext, job: &Arc<Job>) -> Cost;
}

/// Specifies hard constraint which operates on route level.
pub trait HardActivityConstraint: Constraint {
    fn check_activity(
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> ActivityCheckResult;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftActivityConstraint: Constraint {
    fn estimate_activity(route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost;
}

/// Specifies result of hard route constraint check.
pub struct RouteCheckResult {
    /// Violation code.
    pub code: Option<i32>,
}

/// Specifies result of hard route constraint check.
pub struct ActivityCheckResult {
    /// Violation code.
    pub code: Option<i32>,
    /// True if further insertions should be attempted.
    pub stopped: bool,
}

enum ConstraintVariant {
    Hard()
}

/// Provides the way to work with multiple constraints.
pub struct ConstraintPipeline {
    constraints: Vec<ConstraintVariant>
}

impl ConstraintPipeline {
    /// Accepts solution with its context.
    pub fn accept_solution(&self, ctx: &SolutionContext) {
        //self.constraints.iter().for_each(|c| c.)
    }

    /// Accepts solution with its context.
    pub fn accept_route(&self, ctx: &RouteContext) {
        unimplemented!()
    }

    /// Adds constraint to collection as last.
    pub fn add(&mut self, constraint: ConstraintVariant) -> &mut ConstraintPipeline {
        self.constraints.push(constraint);
        self
    }
}
