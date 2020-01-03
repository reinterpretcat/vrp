#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/pipeline_test.rs"]
mod pipeline_test;

use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::Job;
use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;

/// Specifies hard constraint which operates on route level.
pub trait HardRouteConstraint {
    /// Estimates activity insertion in specific route.
    /// Returns violation error if constraint is violated.
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation>;
}

/// Specifies soft constraint which operates on route level.
pub trait SoftRouteConstraint {
    /// Estimates activity insertion in specific route.
    /// Returns non-zero penalty if constraint is violated: positive makes insertion less attractive,
    /// negative - more.
    fn estimate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Cost;
}

/// Specifies hard constraint which operates on activity level.
pub trait HardActivityConstraint {
    /// Estimates activity insertion in specific route leg.
    /// Returns violation error if constraint is violated.
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation>;
}

/// Specifies soft constraint which operates on activity level.
pub trait SoftActivityConstraint {
    /// Estimates activity insertion in specific route leg.
    /// Returns non-zero penalty if constraint is violated: positive makes insertion less attractive,
    /// negative - more.
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost;
}

/// Specifies result of hard route constraint check.
#[derive(Clone, Debug)]
pub struct RouteConstraintViolation {
    /// Violation code which is used as marker of specific constraint violated.
    pub code: i32,
}

/// Specifies result of hard route constraint check.
#[derive(Clone, Debug)]
pub struct ActivityConstraintViolation {
    /// Violation code which is used as marker of specific constraint violated.
    pub code: i32,
    /// True if further insertions should not be attempted.
    pub stopped: bool,
}

/// A variant type for constraint types.
pub enum ConstraintVariant {
    HardRoute(Arc<dyn HardRouteConstraint + Send + Sync>),
    HardActivity(Arc<dyn HardActivityConstraint + Send + Sync>),
    SoftRoute(Arc<dyn SoftRouteConstraint + Send + Sync>),
    SoftActivity(Arc<dyn SoftActivityConstraint + Send + Sync>),
}

/// Represents a constraint module which can be added to constraint pipeline.
pub trait ConstraintModule {
    /// Accept insertion of specific job.
    /// Called once job has been inserted into solution represented via `solution_ctx`,
    /// target route is `route_ctx`, inserted job is `job`.
    /// This method should call `accept_route_state` internally.
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Arc<Job>);

    /// Accept route and updates its state to allow more efficient constraint checks.
    fn accept_route_state(&self, ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// This method called twice: before insertion of all jobs starts and when it ends.
    fn accept_solution_state(&self, ctx: &mut SolutionContext);

    /// Returns unique constraint state keys.
    /// Used to avoid state key interference.
    fn state_keys(&self) -> Iter<i32>;

    /// Returns list of constraints to be used.
    fn get_constraints(&self) -> Iter<ConstraintVariant>;
}

/// Provides the way to work with multiple constraints.
pub struct ConstraintPipeline {
    modules: Vec<Box<dyn ConstraintModule + Send + Sync>>,
    state_keys: HashSet<i32>,
    hard_route_constraints: Vec<Arc<dyn HardRouteConstraint + Send + Sync>>,
    hard_activity_constraints: Vec<Arc<dyn HardActivityConstraint + Send + Sync>>,
    soft_route_constraints: Vec<Arc<dyn SoftRouteConstraint + Send + Sync>>,
    soft_activity_constraints: Vec<Arc<dyn SoftActivityConstraint + Send + Sync>>,
}

impl Default for ConstraintPipeline {
    fn default() -> Self {
        ConstraintPipeline {
            modules: vec![],
            state_keys: Default::default(),
            hard_route_constraints: vec![],
            hard_activity_constraints: vec![],
            soft_route_constraints: vec![],
            soft_activity_constraints: vec![],
        }
    }
}

impl ConstraintPipeline {
    /// Accepts job insertion.
    pub fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, job: &Arc<Job>) {
        self.modules.iter().for_each(|c| c.accept_insertion(solution_ctx, route_ctx, job))
    }

    /// Accepts route state.
    pub fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.modules.iter().for_each(|c| c.accept_route_state(ctx))
    }

    /// Accepts solution state.
    pub fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.modules.iter().for_each(|c| c.accept_solution_state(ctx))
    }

    /// Adds constraint module.
    pub fn add_module(&mut self, module: Box<dyn ConstraintModule + Send + Sync>) -> &mut Self {
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

        self.modules.push(module);

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

impl PartialEq<RouteConstraintViolation> for RouteConstraintViolation {
    fn eq(&self, other: &RouteConstraintViolation) -> bool {
        self.code == other.code
    }
}

impl Eq for RouteConstraintViolation {}

impl PartialEq<ActivityConstraintViolation> for ActivityConstraintViolation {
    fn eq(&self, other: &ActivityConstraintViolation) -> bool {
        self.code == other.code && self.stopped == other.stopped
    }
}

impl Eq for ActivityConstraintViolation {}
