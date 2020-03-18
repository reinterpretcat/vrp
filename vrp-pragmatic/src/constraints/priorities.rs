use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{ActivityContext, RouteContext, SolutionContext};
use vrp_core::models::common::Cost;
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::Job;

/** Adds some extra penalty to jobs with priority bigger than 1. */
pub struct PriorityModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl PriorityModule {
    pub fn new(extra_cost: Cost, code: i32) -> Self {
        Self {
            constraints: vec![
                ConstraintVariant::SoftRoute(Arc::new(PrioritySoftRouteConstraint { extra_cost })),
                ConstraintVariant::HardActivity(Arc::new(PriorityHardActivityConstraint { code })),
            ],
            keys: vec![],
        }
    }
}

impl ConstraintModule for PriorityModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct PrioritySoftRouteConstraint {
    extra_cost: Cost,
}

impl SoftRouteConstraint for PrioritySoftRouteConstraint {
    fn estimate_job(&self, _: &SolutionContext, _: &RouteContext, job: &Job) -> f64 {
        get_priority(job).map_or(0., |priority| ((priority - 1) as f64 * self.extra_cost.max(0.)))
    }
}

struct PriorityHardActivityConstraint {
    code: i32,
}

impl PriorityHardActivityConstraint {
    fn check_priorities(&self, first: &Job, second: &Job, stopped: bool) -> Option<ActivityConstraintViolation> {
        let result = get_priority(first).unwrap_or(1) <= get_priority(second).unwrap_or(1);

        if result {
            None
        } else {
            Some(ActivityConstraintViolation { code: self.code, stopped })
        }
    }
}

impl HardActivityConstraint for PriorityHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let prev = activity_ctx.prev.retrieve_job();
        let target = activity_ctx.target.retrieve_job();
        let next = activity_ctx.next.and_then(|next| next.retrieve_job());

        // TODO last patterns don't work in all cases with with break/reloads?

        match (prev, target, next) {
            (_, None, _) => None,
            (None, Some(_), None) => None,
            (Some(prev), Some(target), _) => self.check_priorities(&prev, &target, false),
            (_, Some(target), Some(next)) => self.check_priorities(&target, &next, true),
        }
    }
}

fn get_priority(job: &Job) -> Option<i32> {
    match job {
        Job::Single(job) => job.dimens.get_value::<i32>("priority"),
        Job::Multi(job) => job.dimens.get_value::<i32>("priority"),
    }
    .cloned()
}
