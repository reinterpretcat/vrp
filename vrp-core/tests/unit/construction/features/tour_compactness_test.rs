use crate::construction::features::create_tour_compactness_feature;
use crate::construction::heuristics::{InsertionContext, MoveContext};
use crate::helpers::models::domain::ProblemBuilder;
use crate::helpers::solver::{generate_matrix_routes_with_defaults, get_job_by_id};
use rosomaxa::prelude::Float;
use rosomaxa::utils::Environment;
use std::sync::Arc;

parameterized_test! {can_count_neighbours_in_route, (routes, job_radius, candidate, expected), {
    can_count_neighbours_in_route_impl(routes, job_radius, candidate, expected);
}}

can_count_neighbours_in_route! {
    // c0 c3 c6      c0 c3
    // c1 c4 c7  or  c1 c4
    // c2 c5 c8      c2 c5
    case_01_far_job:                ((3, 3), 3, (0, "c8"), (3., 15. / 2.)),
    case_02_near_job:               ((3, 2), 3, (0, "c0"), (2., 10. / 2.)),
    case_03_near_job:               ((3, 2), 3, (0, "c1"), (1., 10. / 2.)),

    // c0 c5
    // c1 c6
    // c2 c7
    // c3 c8
    // c4 c9
    case_04_near_job_larger_routes: ((5, 2), 5, (0, "c0"), (3., 30. / 2.)),
}

fn can_count_neighbours_in_route_impl(
    routes: (usize, usize),
    job_radius: usize,
    candidate: (usize, &str),
    expected: (Float, Float),
) {
    let (route_idx, job_id) = candidate;
    let (expected_estimation, expected_fitness) = expected;
    let (rows, cols) = routes;
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(rows, cols, false);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    let feature = create_tour_compactness_feature("compact", insertion_ctx.problem.jobs.clone(), job_radius)
        .expect("cannot create feature");
    let (state, objective) = { (feature.state.as_ref().unwrap(), feature.objective.as_ref().unwrap()) };

    state.accept_solution_state(&mut insertion_ctx.solution);
    let fitness = objective.fitness(&insertion_ctx);
    let estimation = objective.estimate(&MoveContext::Route {
        solution_ctx: &insertion_ctx.solution,
        route_ctx: &insertion_ctx.solution.routes[route_idx],
        job: get_job_by_id(&insertion_ctx, job_id).unwrap(),
    });

    assert_eq!(estimation, expected_estimation);
    assert_eq!(fitness, expected_fitness);
}

#[test]
fn can_return_err_if_feature_cannot_be_created() {
    let jobs = ProblemBuilder::default().build().jobs;

    let result = create_tour_compactness_feature("compact", jobs, 0);

    assert!(result.is_err());
}
