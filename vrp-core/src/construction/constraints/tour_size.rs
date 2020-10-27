#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/tour_size_test.rs"]
mod tour_size_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::{Actor, Job};
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// A function which returns tour size limit for given actor.
pub type TourSizeResolver = Arc<dyn Fn(&Actor) -> Option<usize> + Sync + Send>;

/// Limits amount of job activities per tour.
pub struct TourSizeModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl TourSizeModule {
    /// Creates a new instance of `TourSizeModule`.
    pub fn new(limit_func: TourSizeResolver, code: i32) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardRoute(Arc::new(TourSizeHardRouteConstraint { limit_func, code }))],
            state_keys: vec![],
        }
    }
}

impl ConstraintModule for TourSizeModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, _: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct TourSizeHardRouteConstraint {
    code: i32,
    limit_func: TourSizeResolver,
}

impl HardRouteConstraint for TourSizeHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(limit) = self.limit_func.deref()(ctx.route.actor.as_ref()) {
            let tour_activities = ctx.route.tour.activity_count();

            let job_activities = match job {
                Job::Single(_) => 1,
                Job::Multi(multi) => multi.jobs.len(),
            };

            if tour_activities + job_activities > limit {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}
