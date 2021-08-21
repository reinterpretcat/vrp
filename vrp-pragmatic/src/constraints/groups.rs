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
    constraints: Vec<ConstraintVariant>,
    state_key: i32,
    keys: Vec<i32>,
}

impl GroupModule {
    /// Creates a new instance of `GroupModule`.
    pub fn new(code: i32, state_key: i32) -> Self {
        Self {
            constraints: vec![ConstraintVariant::HardRoute(Arc::new(GroupHardRouteConstraint { code, state_key }))],
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


        // TODO didn't work with decompose search as it splits solution into multiple sub-solutions!

        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            let current_jobs_count = route_ctx.route.tour.job_count();
            let groups = get_groups(route_ctx);
            route_ctx.state_mut().put_route_state(self.state_key, (groups, current_jobs_count));
        });
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct GroupHardRouteConstraint {
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

            if other_route {
                Some(RouteConstraintViolation { code: self.code })
            } else {
                None
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
