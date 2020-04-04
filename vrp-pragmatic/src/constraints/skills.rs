use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::Job;

pub struct SkillsModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl SkillsModule {
    pub fn new(code: i32) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardRoute(Arc::new(SkillsHardRouteConstraint { code }))],
            keys: vec![],
        }
    }
}

impl ConstraintModule for SkillsModule {
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

struct SkillsHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for SkillsHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(requirement) = job.dimens().get_value::<HashSet<String>>("skills") {
            if let Some(skills) = ctx.route.actor.vehicle.dimens.get_value::<HashSet<String>>("skills") {
                if requirement.is_subset(skills) {
                    return None;
                }
            }

            Some(RouteConstraintViolation { code: self.code })
        } else {
            None
        }
    }
}
