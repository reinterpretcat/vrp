#[cfg(test)]
#[path = "../../tests/unit/constraints/skills_test.rs"]
mod skills_test;

use hashbrown::HashSet;
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
    code: i32,
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl SkillsModule {
    pub fn new(code: i32) -> Self {
        Self {
            code,
            constraints: vec![ConstraintVariant::HardRoute(Arc::new(SkillsHardRouteConstraint { code }))],
            keys: vec![],
        }
    }
}

impl ConstraintModule for SkillsModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        let source_skills = get_skills(&source);
        let candidate_skills = get_skills(&candidate);

        let check_skill_sets = |source_set: Option<&HashSet<String>>, candidate_set: Option<&HashSet<String>>| match (
            source_set,
            candidate_set,
        ) {
            (Some(_), None) | (None, None) => true,
            (None, Some(_)) => false,
            (Some(source_skills), Some(candidate_skills)) => candidate_skills.is_subset(source_skills),
        };

        let has_comparable_skills = match (source_skills, candidate_skills) {
            (Some(_), None) | (None, None) => true,
            (None, Some(_)) => false,
            (Some(source_skills), Some(candidate_skills)) => {
                check_skill_sets(source_skills.all_of.as_ref(), candidate_skills.all_of.as_ref())
                    && check_skill_sets(source_skills.one_of.as_ref(), candidate_skills.one_of.as_ref())
                    && check_skill_sets(source_skills.none_of.as_ref(), candidate_skills.none_of.as_ref())
            }
        };

        if has_comparable_skills {
            Ok(source)
        } else {
            Err(self.code)
        }
    }

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
        if let Some(job_skills) = get_skills(job) {
            let vehicle_skills = ctx.route.actor.vehicle.dimens.get_value::<HashSet<String>>("skills");
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

fn get_skills(job: &Job) -> Option<&JobSkills> {
    job.dimens().get_value::<JobSkills>("skills")
}
