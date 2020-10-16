#[cfg(test)]
#[path = "../../tests/unit/constraints/skills_test.rs"]
mod skills_test;

use std::collections::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::Job;

/// A job skills limitation for a vehicle.
pub struct JobSkills {
    /// Vehicle should have all of these skills defined.
    pub all_of: Option<HashSet<String>>,
    /// Vehicle should have at least one of these skills defined.
    pub one_of: Option<HashSet<String>>,
    /// Vehicle should have none of these skills defined.
    pub none_of: Option<HashSet<String>>,
}

/// A skills module provides way to control jobs/vehicle assignment.
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
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {}

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
        let job_skills = job.dimens().get_value::<JobSkills>("skills");
        let vehicle_skills = ctx.route.actor.vehicle.dimens.get_value::<HashSet<String>>("skills");

        if let Some(job_skills) = job_skills {
            let is_ok = check_all_of(job_skills, &vehicle_skills)
                && check_one_of(job_skills, &vehicle_skills)
                && check_none_of(job_skills, &vehicle_skills);
            if !is_ok {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}

fn check_all_of(job_skills: &JobSkills, vehicle_skills: &Option<&HashSet<String>>) -> bool {
    match (job_skills.all_of.as_ref(), vehicle_skills) {
        (Some(job_skills), Some(vehicle_skills)) => job_skills.is_subset(vehicle_skills),
        (Some(skills), None) if skills.is_empty() => true,
        (Some(_), None) => false,
        _ => true,
    }
}

fn check_one_of(job_skills: &JobSkills, vehicle_skills: &Option<&HashSet<String>>) -> bool {
    match (job_skills.one_of.as_ref(), vehicle_skills) {
        (Some(job_skills), Some(vehicle_skills)) => job_skills.iter().any(|skill| vehicle_skills.contains(skill)),
        (Some(skills), None) if skills.is_empty() => true,
        (Some(_), None) => false,
        _ => true,
    }
}

fn check_none_of(job_skills: &JobSkills, vehicle_skills: &Option<&HashSet<String>>) -> bool {
    match (job_skills.none_of.as_ref(), vehicle_skills) {
        (Some(job_skills), Some(vehicle_skills)) => job_skills.is_disjoint(vehicle_skills),
        _ => true,
    }
}
