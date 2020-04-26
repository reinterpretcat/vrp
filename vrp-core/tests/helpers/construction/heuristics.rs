use crate::construction::constraints::ConstraintPipeline;
use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
use crate::helpers::models::domain::{create_empty_problem_with_constraint, create_empty_solution_context};
use crate::models::solution::Registry;
use crate::utils::DefaultRandom;
use std::sync::Arc;

pub fn create_insertion_context(
    registry: Registry,
    constraint: ConstraintPipeline,
    routes: Vec<RouteContext>,
) -> InsertionContext {
    InsertionContext {
        problem: create_empty_problem_with_constraint(constraint),
        solution: SolutionContext { routes, registry, ..create_empty_solution_context() },
        random: Arc::new(DefaultRandom::default()),
    }
}

pub fn create_test_insertion_context(registry: Registry) -> InsertionContext {
    let routes: Vec<RouteContext> = vec![RouteContext::new(registry.next().next().unwrap())];
    let constraint = create_constraint_pipeline_with_transport();
    create_insertion_context(registry, constraint, routes)
}
