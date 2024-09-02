use super::*;
use crate::helpers::construction::clustering::dbscan::create_test_distances;
use crate::helpers::models::domain::{ProblemBuilder, TestGoalContextBuilder};
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::*;
use rosomaxa::prelude::Environment;
use std::sync::Arc;

#[test]
fn can_create_ruin_cluster_with_default_params() {
    let environment = Arc::new(Environment::default());
    let (problem, _) = generate_matrix_routes(
        8,
        1,
        false,
        |_, _, _| TestGoalContextBuilder::with_transport_feature().build(),
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |_| (vec![0; 64], create_test_distances(1.)),
    );

    let removal = ClusterRemoval::new_with_defaults(Arc::new(problem), environment);

    assert!(!removal.clusters.is_empty());
}

#[test]
fn can_handle_empty_problem() {
    let problem = Arc::new(ProblemBuilder::default().build());
    let limits = RemovalLimits::new(&problem);

    let removal = ClusterRemoval::new(problem, Arc::new(Environment::default()), 3, limits);

    assert!(removal.clusters.is_empty());
}

parameterized_test! {can_ruin_jobs, (limit, cluster_size, expected), {
    can_ruin_jobs_impl(limit, cluster_size, expected);
}}

can_ruin_jobs! {
    case_01: (4, 3, 4),
    case_02: (5, 3, 4),
    case_03: (8, 3, 4),
}

fn can_ruin_jobs_impl(limit: usize, min_items: usize, expected: usize) {
    let limits = RemovalLimits { removed_activities_range: limit..limit, affected_routes_range: 8..8 };
    let (problem, solution) = generate_matrix_routes(
        8,
        1,
        false,
        |_, _, _| TestGoalContextBuilder::with_transport_feature().build(),
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |_| (vec![0; 64], create_test_distances(1.)),
    );
    let problem = Arc::new(problem);
    let environment = Arc::new(Environment::default());
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());

    let insertion_ctx = ClusterRemoval::new(problem, environment, min_items, limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), 0);
    assert_eq!(insertion_ctx.solution.locked.len(), 0);
    assert_eq!(insertion_ctx.solution.required.len(), expected);
    assert_eq!(
        insertion_ctx.solution.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum::<usize>(),
        8 - expected
    );
}
