#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/pipeline_test.rs"]
mod pipeline_test;

use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::Solution;
use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;

/// Specifies a base constraint behavior.
pub trait Constraint {
    /// Accept route and updates its state to allow more efficient constraint checks.
    /// Called in thread-safe context, so it is a chance to apply some changes.
    fn accept_route(&self, ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// Called in thread-safe context.
    fn accept_solution(&self, ctx: &mut SolutionContext);

    /// Returns unique constraint state keys.
    fn state_keys(&self) -> Iter<i32>;
}

/// Specifies hard constraint which operates on route level.
pub trait HardRouteConstraint: Constraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation>;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftRouteConstraint: Constraint {
    fn estimate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Cost;
}

/// Specifies hard constraint which operates on route level.
pub trait HardActivityConstraint: Constraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation>;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftActivityConstraint: Constraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost;
}

/// Specifies result of hard route constraint check.
#[derive(Clone)]
pub struct RouteConstraintViolation {
    /// Violation code.
    pub code: i32,
}

/// Specifies result of hard route constraint check.
#[derive(Clone)]
pub struct ActivityConstraintViolation {
    /// Violation code.
    pub code: i32,
    /// True if further insertions should be attempted.
    pub stopped: bool,
}

/// Provides the way to work with multiple constraints.
pub struct ConstraintPipeline {
    constraints: Vec<Arc<dyn Constraint>>,
    keys: HashSet<i32>,
    hard_route_constraints: Vec<Arc<dyn HardRouteConstraint>>,
    hard_activity_constraints: Vec<Arc<dyn HardActivityConstraint>>,
    soft_route_constraints: Vec<Arc<dyn SoftRouteConstraint>>,
    soft_activity_constraints: Vec<Arc<dyn SoftActivityConstraint>>,
}

impl ConstraintPipeline {
    pub fn new() -> Self {
        ConstraintPipeline {
            constraints: vec![],
            keys: Default::default(),
            hard_route_constraints: vec![],
            hard_activity_constraints: vec![],
            soft_route_constraints: vec![],
            soft_activity_constraints: vec![],
        }
    }

    /// Accepts solution with its context.
    pub fn accept_solution(&self, ctx: &mut SolutionContext) {
        self.constraints.iter().for_each(|c| c.accept_solution(ctx))
    }

    /// Accepts solution with its context.
    pub fn accept_route(&self, ctx: &mut RouteContext) {
        self.constraints.iter().for_each(|c| c.accept_route(ctx))
    }

    /// Adds constraint to collection as last.
    pub fn add_constraint(&mut self, constraint: &Arc<dyn Constraint>) -> &mut Self {
        constraint.state_keys().for_each(|key| {
            if let Some(duplicate) = self.keys.get(key) {
                panic!(
                    "Attempt to register constraint with key duplication: {}",
                    duplicate
                )
            }
            self.keys.insert(key.clone());
        });
        self.constraints.push(constraint.clone());
        self
    }

    /// Adds hard route constraint to collection as last.
    pub fn add_hard_route(
        &mut self,
        constraint: &Arc<impl HardRouteConstraint + 'static>,
    ) -> &mut Self {
        self.hard_route_constraints.push(constraint.clone());
        self
    }

    /// Adds hard activity constraint to collection as last.
    pub fn add_hard_activity(
        &mut self,
        constraint: &Arc<impl HardActivityConstraint + 'static>,
    ) -> &mut Self {
        self.hard_activity_constraints.push(constraint.clone());
        self
    }

    /// Adds soft route constraint to collection as last.
    pub fn add_soft_route(
        &mut self,
        constraint: &Arc<impl SoftRouteConstraint + 'static>,
    ) -> &mut Self {
        self.soft_route_constraints.push(constraint.clone());
        self
    }

    /// Adds soft activity constraint to collection as last.
    pub fn add_soft_activity(
        &mut self,
        constraint: &Arc<impl SoftActivityConstraint + 'static>,
    ) -> &mut Self {
        self.soft_activity_constraints.push(constraint.clone());
        self
    }

    /// Checks whether all hard route constraints are fulfilled.
    /// Returns result of first failed constraint or empty value.
    pub fn evaluate_hard_route(
        &self,
        ctx: &RouteContext,
        job: &Arc<Job>,
    ) -> Option<RouteConstraintViolation> {
        self.hard_route_constraints
            .iter()
            .find_map(|c| c.evaluate_job(ctx, job))
    }

    /// Checks whether all activity route constraints are fulfilled.
    /// Returns result of first failed constraint or empty value.
    pub fn evaluate_hard_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.hard_activity_constraints
            .iter()
            .find_map(|c| c.evaluate_activity(route_ctx, activity_ctx))
    }

    /// Checks soft route constraints and aggregates associated actual and penalty costs.
    pub fn evaluate_soft_route(&self, ctx: &RouteContext, job: &Arc<Job>) -> Cost {
        self.soft_route_constraints
            .iter()
            .map(|c| c.estimate_job(ctx, job))
            .sum()
    }

    /// Checks soft route constraints and aggregates associated actual and penalty costs.
    pub fn evaluate_soft_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Cost {
        self.soft_activity_constraints
            .iter()
            .map(|c| c.estimate_activity(route_ctx, activity_ctx))
            .sum()
    }
}
