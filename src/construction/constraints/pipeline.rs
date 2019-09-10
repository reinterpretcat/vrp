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

/// Specifies hard constraint which operates on route level.
pub trait HardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation>;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftRouteConstraint {
    fn estimate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Cost;
}

/// Specifies hard constraint which operates on route level.
pub trait HardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation>;
}

/// Specifies hard constraint which operates on route level.
pub trait SoftActivityConstraint {
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

/// A variant for constraint types.
pub enum ConstraintVariant {
    HardRoute(Arc<dyn HardRouteConstraint>),
    HardActivity(Arc<dyn HardActivityConstraint>),
    SoftRoute(Arc<dyn SoftRouteConstraint>),
    SoftActivity(Arc<dyn SoftActivityConstraint>),
}

/// Represents constraint module which can be added to constraint pipeline.
pub trait ConstraintModule {
    /// Accept route and updates its state to allow more efficient constraint checks.
    fn accept_route_state(&self, ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    fn accept_solution_state(&self, ctx: &mut SolutionContext);

    /// Returns unique constraint state keys.
    fn state_keys(&self) -> Iter<i32>;

    /// Returns list of constraints.
    fn get_constraints(&self) -> Iter<ConstraintVariant>;
}

/// Provides the way to work with multiple constraints.
pub struct ConstraintPipeline {
    modules: Vec<Arc<dyn ConstraintModule>>,
    state_keys: HashSet<i32>,
    hard_route_constraints: Vec<Arc<dyn HardRouteConstraint>>,
    hard_activity_constraints: Vec<Arc<dyn HardActivityConstraint>>,
    soft_route_constraints: Vec<Arc<dyn SoftRouteConstraint>>,
    soft_activity_constraints: Vec<Arc<dyn SoftActivityConstraint>>,
}

impl ConstraintPipeline {
    pub fn new() -> Self {
        ConstraintPipeline {
            modules: vec![],
            state_keys: Default::default(),
            hard_route_constraints: vec![],
            hard_activity_constraints: vec![],
            soft_route_constraints: vec![],
            soft_activity_constraints: vec![],
        }
    }

    /// Accepts solution with its context.
    pub fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.modules.iter().for_each(|c| c.accept_solution_state(ctx))
    }

    /// Accepts solution with its context.
    pub fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.modules.iter().for_each(|c| c.accept_route_state(ctx))
    }

    /// Adds constraint module.
    pub fn add_module(&mut self, module: impl ConstraintModule) -> &mut Self {
        module.state_keys().for_each(|key| {
            if let Some(duplicate) = self.state_keys.get(key) {
                panic!("Attempt to register constraint with key duplication: {}", duplicate)
            }
            self.state_keys.insert(key.clone());
        });

        module.get_constraints().for_each(|c| match c {
            ConstraintVariant::HardRoute(c) => self.hard_route_constraints.push(c.clone()),
            ConstraintVariant::HardActivity(c) => self.hard_activity_constraints.push(c.clone()),
            ConstraintVariant::SoftRoute(c) => self.soft_route_constraints.push(c.clone()),
            ConstraintVariant::SoftActivity(c) => self.soft_activity_constraints.push(c.clone()),
        });

        self
    }

    /// Checks whether all hard route constraints are fulfilled.
    /// Returns result of first failed constraint or empty value.
    pub fn evaluate_hard_route(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        self.hard_route_constraints.iter().find_map(|c| c.evaluate_job(ctx, job))
    }

    /// Checks whether all activity route constraints are fulfilled.
    /// Returns result of first failed constraint or empty value.
    pub fn evaluate_hard_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.hard_activity_constraints.iter().find_map(|c| c.evaluate_activity(route_ctx, activity_ctx))
    }

    /// Checks soft route constraints and aggregates associated actual and penalty costs.
    pub fn evaluate_soft_route(&self, ctx: &RouteContext, job: &Arc<Job>) -> Cost {
        self.soft_route_constraints.iter().map(|c| c.estimate_job(ctx, job)).sum()
    }

    /// Checks soft route constraints and aggregates associated actual and penalty costs.
    pub fn evaluate_soft_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost {
        self.soft_activity_constraints.iter().map(|c| c.estimate_activity(route_ctx, activity_ctx)).sum()
    }
}
