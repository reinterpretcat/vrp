use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::models::solution::Activity;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;

/// Represents a shared resource.
pub trait SharedResource: Add + Sub + Sized + Send + Sync {}

/// Provides way to define and use shared across multiple routes resource.
pub struct SharedResourceModule<T: SharedResource> {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    resource_demand_fn: Arc<dyn Fn(&Job, &Activity) -> Option<T> + Send + Sync>,
    resource_capacity_fn: Arc<dyn Fn(&Job) -> Option<T> + Send + Sync>,
    interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
}

impl<T: SharedResource> SharedResourceModule<T> {
    /// Creates a new instance of `SharedResourceModule`.
    pub fn new(
        code: i32,
        state_key: i32,
        resource_demand_fn: Arc<dyn Fn(&Job, &Activity) -> Option<T> + Send + Sync>,
        resource_capacity_fn: Arc<dyn Fn(&Job) -> Option<T> + Send + Sync>,
        interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
    ) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(SharedResourceHardActivityConstraint {
                code,
                interval_fn: interval_fn.clone(),
            }))],
            state_keys: vec![state_key],
            resource_demand_fn,
            resource_capacity_fn,
            interval_fn,
        }
    }
}

impl<T: SharedResource> ConstraintModule for SharedResourceModule<T> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        // TODO save consumption of each shared resource in the route state
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {
        // TODO iterate through all route states, read their resource consumption and store
        //      aggregated consumption in the solution state
    }

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct SharedResourceHardActivityConstraint {
    code: i32,
    interval_fn: Arc<dyn Fn(&RouteContext) -> &[(usize, usize)] + Send + Sync>,
}

impl HardActivityConstraint for SharedResourceHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        // NOTE: we cannot pass SolutionContext here as it should be not needed by evaluate_activity.

        self.interval_fn.deref()(route_ctx)
            .iter()
            // TODO what's about OVRP?
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .map(|(start_idx, end_idx)| {

                unimplemented!()
            });

        // TODO
        //      get job resource consumption value
        //      get available resource value from route context
        unimplemented!()
    }
}
