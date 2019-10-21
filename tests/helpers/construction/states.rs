use crate::construction::constraints::ConstraintPipeline;
use crate::construction::states::{InsertionContext, InsertionProgress, RouteContext, SolutionContext};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::models::domain::create_empty_problem_with_constraint;
use crate::models::solution::Registry;
use crate::utils::DefaultRandom;
use std::sync::Arc;

pub fn test_insertion_progress() -> InsertionProgress {
    InsertionProgress { cost: Some(1000.0), completeness: 1.0, total: 1 }
}

pub fn create_insertion_context(
    registry: Registry,
    constraint: ConstraintPipeline,
    routes: Vec<RouteContext>,
) -> InsertionContext {
    InsertionContext {
        progress: test_insertion_progress(),
        problem: create_empty_problem_with_constraint(constraint),
        solution: SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            routes,
            registry,
        },
        locked: Arc::new(Default::default()),
        random: Arc::new(DefaultRandom::new()),
    }
}

pub fn create_test_insertion_context(registry: Registry) -> InsertionContext {
    let mut routes: Vec<RouteContext> = vec![RouteContext::new(registry.next().next().unwrap())];
    let mut constraint = create_constraint_pipeline_with_timing();
    create_insertion_context(registry, constraint, routes)
}
