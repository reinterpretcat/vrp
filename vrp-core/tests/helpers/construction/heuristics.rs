use crate::construction::heuristics::{InsertionContext, RegistryContext, RouteContext, SolutionContext};
use crate::helpers::construction::features::create_goal_ctx_with_transport;
use crate::helpers::models::domain::{create_empty_problem_with_goal_ctx, create_empty_solution_context};
use crate::models::solution::Registry;
use crate::models::GoalContext;
use rosomaxa::prelude::Environment;
use std::sync::Arc;

pub fn create_insertion_context(
    registry: Registry,
    goal_ctx: GoalContext,
    routes: Vec<RouteContext>,
) -> InsertionContext {
    let problem = create_empty_problem_with_goal_ctx(goal_ctx);

    let mut registry = registry;
    routes.iter().for_each(|route_ctx| {
        registry.use_actor(&route_ctx.route.actor);
    });
    let registry = RegistryContext::new(problem.goal.clone(), registry);

    InsertionContext {
        problem,
        solution: SolutionContext { routes, registry, ..create_empty_solution_context() },
        environment: Arc::new(Environment::default()),
    }
}

pub fn create_test_insertion_context(registry: Registry) -> InsertionContext {
    let routes: Vec<RouteContext> = vec![RouteContext::new(registry.next().next().unwrap())];
    let goal_ctx = create_goal_ctx_with_transport();
    create_insertion_context(registry, goal_ctx, routes)
}
