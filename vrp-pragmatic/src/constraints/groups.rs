#[cfg(test)]
#[path = "../../tests/unit/constraints/group_test.rs"]
mod group_test;

use hashbrown::HashSet;
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::ValueDimension;
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
        if let Some(group) = get_group(job) {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
            let jobs_count = route_ctx.route.tour.job_count();
            let mut groups = get_groups(route_ctx);
            groups.insert(group.clone());
            route_ctx.state_mut().put_route_state(self.state_key, (groups, jobs_count))
        }
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let current_jobs_count = ctx.route.tour.job_count();
        let old_jobs_count = ctx
            .state
            .get_route_state::<(HashSet<String>, usize)>(self.state_key)
            .map(|(_, jobs)| *jobs)
            .unwrap_or(current_jobs_count);

        if old_jobs_count != current_jobs_count {
            let groups = get_groups(ctx);
            ctx.state_mut().put_route_state(self.state_key, (groups, current_jobs_count))
        }
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        // NOTE we can filter here by stale flag, but then we need to keep non-changed route
        // cache and identify routes which where deleted to remove them from cache. Instead,
        // let's go through all routes and create evaluation cache from scratch. However, this
        // approach has performance implications for calling `accept_solution_state` method frequently.

        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            let current_jobs_count = route_ctx.route.tour.job_count();
            let groups = get_groups(route_ctx);
            route_ctx.state_mut().put_route_state(self.state_key, (groups, current_jobs_count));
        });
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        match (get_group(&source), get_group(&candidate)) {
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
        get_group(job).and_then(|group| {
            let other_route = solution_ctx
                .routes
                .iter()
                .filter(|rc| rc.route.actor != route_ctx.route.actor)
                .filter_map(|rc| rc.state.get_route_state::<(HashSet<String>, usize)>(self.state_key))
                .any(|(groups, _)| groups.contains(group));

            let current_route = route_ctx
                .state
                .get_route_state::<(HashSet<String>, usize)>(self.state_key)
                .map_or(false, |(groups, _)| groups.contains(group));

            match (other_route, current_route) {
                (true, _) => Some(RouteConstraintViolation { code: self.code }),
                // NOTE handle partial solution context use case (e.g. decompose search)
                (false, false) => {
                    let is_full_problem = amount_of_jobs(solution_ctx) == self.total_jobs;
                    if is_full_problem {
                        None
                    } else {
                        Some(RouteConstraintViolation { code: self.code })
                    }
                }
                (false, true) => None,
            }
        })
    }
}

fn get_group(job: &Job) -> Option<&String> {
    job.dimens().get_value::<String>("group")
}

fn get_groups(route_ctx: &RouteContext) -> HashSet<String> {
    route_ctx.route.tour.jobs().filter_map(|job| get_group(&job).cloned()).collect()
}

fn amount_of_jobs(solution_ctx: &SolutionContext) -> usize {
    let assigned = solution_ctx.routes.iter().map(|route_ctx| route_ctx.route.tour.job_count()).sum::<usize>();

    let required = solution_ctx.required.iter().filter(|job| !solution_ctx.unassigned.contains_key(job)).count();

    solution_ctx.unassigned.len() + required + solution_ctx.ignored.len() + assigned
}
