#[cfg(test)]
#[path = "../../tests/unit/constraints/group_test.rs"]
mod group_test;

use crate::construction::enablers::JobTie;
use hashbrown::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::problem::Job;

/// A group module provides the way to stick certain jobs to the same tour.
pub struct GroupModule {
    code: i32,
    constraints: Vec<ConstraintVariant>,
    state_key: i32,
    keys: Vec<i32>,
}

impl GroupModule {
    /// Creates a new instance of `GroupModule`.
    pub fn new(total_jobs: usize, code: i32, state_key: i32) -> Self {
        Self {
            code,
            constraints: vec![ConstraintVariant::HardRoute(Arc::new(GroupHardRouteConstraint {
                total_jobs,
                code,
                state_key,
            }))],
            state_key,
            keys: vec![state_key],
        }
    }
}

impl ConstraintModule for GroupModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if let Some(group) = job.dimens().get_job_group() {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();

            let mut groups = get_groups(route_ctx);
            groups.insert(group.clone());

            route_ctx.state_mut().put_route_state(self.state_key, groups)
        }
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let groups = get_groups(route_ctx);
            route_ctx.state_mut().put_route_state(self.state_key, groups);
        });
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        match (source.dimens().get_job_group(), candidate.dimens().get_job_group()) {
            (None, None) => Ok(source),
            (Some(s_group), Some(c_group)) if s_group == c_group => Ok(source),
            _ => Err(self.code),
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct GroupHardRouteConstraint {
    total_jobs: usize,
    code: i32,
    state_key: i32,
}

impl HardRouteConstraint for GroupHardRouteConstraint {
    fn evaluate_job(
        &self,
        solution_ctx: &SolutionContext,
        route_ctx: &RouteContext,
        job: &Job,
    ) -> Option<RouteConstraintViolation> {
        job.dimens().get_job_group().and_then(|group| {
            let is_partial_problem = solution_ctx.get_jobs_amount() != self.total_jobs;
            if is_partial_problem {
                return Some(RouteConstraintViolation { code: self.code });
            }

            let other_route = solution_ctx
                .routes
                .iter()
                .filter(|rc| rc.route.actor != route_ctx.route.actor)
                .filter_map(|rc| rc.state.get_route_state::<HashSet<String>>(self.state_key))
                .any(|groups| groups.contains(group));

            if other_route {
                Some(RouteConstraintViolation { code: self.code })
            } else {
                None
            }
        })
    }
}

fn get_groups(route_ctx: &RouteContext) -> HashSet<String> {
    route_ctx.route.tour.jobs().filter_map(|job| job.dimens().get_job_group().cloned()).collect()
}
