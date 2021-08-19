#[cfg(test)]
#[path = "../../tests/unit/constraints/group_test.rs"]
mod group_test;

use hashbrown::{HashMap, HashSet};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::ValueDimension;
use vrp_core::models::problem::{Actor, Job};
use vrp_core::utils::as_mut;

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
            let actor = solution_ctx.routes[route_index].route.actor.clone();
            if let Some(actor_groups) = get_actor_groups(solution_ctx, self.state_key) {
                unsafe { as_mut(actor_groups) }.insert(group.clone(), actor);
            } else {
                let actor_groups = std::iter::once((group.clone(), actor)).collect::<HashMap<_, _>>();
                solution_ctx.state.insert(self.state_key, Arc::new(actor_groups));
            }
        }
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        // NOTE we can filter here by stale flag, but then we need to keep non-changed route
        // cache and identify routes which where deleted to remove them from cache. Instead,
        // let's go through all routes and create evaluation cache from scratch. However, this
        // approach has performance implications for calling `accept_solution_state` method frequently.

        let actor_groups: HashMap<_, _> = solution_ctx
            .routes
            .iter_mut()
            .map(|route_ctx| {
                let groups =
                    route_ctx.route.tour.jobs().filter_map(|job| get_group(&job).cloned()).collect::<HashSet<_>>();
                (route_ctx.route.actor.clone(), groups)
            })
            .fold(HashMap::default(), |mut acc, (actor, groups)| {
                groups.into_iter().for_each(|group| {
                    acc.insert(group, actor.clone());
                });

                acc
            });

        // update evaluation cache
        solution_ctx.state.insert(self.state_key, Arc::new(actor_groups));
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
        get_group(job)
            .zip(
                solution_ctx
                    .state
                    .get(&self.state_key)
                    .and_then(|value| value.downcast_ref::<HashMap<String, Arc<Actor>>>()),
            )
            .and_then(|(group, groups)| {
                groups.get(group).and_then(|actor| {
                    if route_ctx.route.actor == *actor {
                        None
                    } else {
                        Some(RouteConstraintViolation { code: self.code })
                    }
                })
            })
    }
}

fn get_group(job: &Job) -> Option<&String> {
    job.dimens().get_value::<String>("group")
}

fn get_actor_groups(solution_ctx: &mut SolutionContext, state_key: i32) -> Option<&HashMap<String, Arc<Actor>>> {
    solution_ctx.state.get_mut(&state_key).and_then(|cache| cache.downcast_ref::<HashMap<String, Arc<Actor>>>())
}
