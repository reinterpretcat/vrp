use crate::construction::features::create_tour_compactness_feature;
use crate::construction::heuristics::{InsertionContext, MoveContext};
use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_problem};
use crate::helpers::solver::{generate_matrix_routes_with_defaults, get_job_by_id};
use crate::models::common::Cost;
use rosomaxa::utils::Environment;
use std::cmp::Ordering;
use std::sync::Arc;

parameterized_test! {can_compare_solutions_with_thresholds, (thresholds, states, expected), {
    can_compare_solutions_with_thresholds_impl(thresholds, states, expected);
}}

can_compare_solutions_with_thresholds! {
    case_01_above_thresholds: (Some((3, 0.)), (5., 10.), Ordering::Less),
    case_02_below_thresholds: (Some((3, 0.1)), (10., 10.), Ordering::Equal),
    case_03_below_min_considered_equal: (Some((10, 0.1)), (9., 5.), Ordering::Equal),

    case_04_above_min_consider_difference: (Some((3, 0.1)), (10., 9.1), Ordering::Equal),
    case_05_above_min_consider_difference: (Some((3, 0.05)), (10., 9.1), Ordering::Greater),
    case_06_above_min_consider_difference: (Some((3, 0.05)), (9.1, 10.), Ordering::Less),
}

fn can_compare_solutions_with_thresholds_impl(
    thresholds: Option<(usize, f64)>,
    states: (f64, f64),
    expected: Ordering,
) {
    let state_key = 1;
    let (left_state, right_state) = states;
    let create_insertion_ctx_fn = |state_value: Cost| {
        let mut insertion_ctx = create_empty_insertion_context();
        insertion_ctx.solution.state.insert(state_key, Arc::new(state_value));
        insertion_ctx
    };
    let objective =
        create_tour_compactness_feature("compact", create_empty_problem().jobs.clone(), 3, state_key, thresholds)
            .expect("cannot create feature")
            .objective
            .unwrap();

    let result = objective.total_order(&create_insertion_ctx_fn(left_state), &create_insertion_ctx_fn(right_state));

    assert_eq!(result, expected);
}

parameterized_test! {can_count_neighbours_in_route, (routes, job_radius, candidate, expected), {
    can_count_neighbours_in_route_impl(routes, job_radius, candidate, expected);
}}

can_count_neighbours_in_route! {
    // c0 c3 c6      c0 c3
    // c1 c4 c7  or  c1 c4
    // c2 c5 c8      c2 c5
    case_01_far_job:                ((3, 3), 3, (0, "c8"), (3., 15.)),
    case_02_near_job:               ((3, 2), 3, (0, "c0"), (2., 10.)),
    case_03_near_job:               ((3, 2), 3, (0, "c1"), (1., 10.)),

    // c0 c5
    // c1 c6
    // c2 c7
    // c3 c8
    // c4 c9
    case_04_near_job_larger_routes: ((5, 2), 5, (0, "c0"), (3., 30.)),
}

fn can_count_neighbours_in_route_impl(
    routes: (usize, usize),
    job_radius: usize,
    candidate: (usize, &str),
    expected: (f64, f64),
) {
    let (route_idx, job_id) = candidate;
    let (expected_estimation, expected_fitness) = expected;
    let (rows, cols) = routes;
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(rows, cols, false);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    let feature = create_tour_compactness_feature("compact", insertion_ctx.problem.jobs.clone(), job_radius, 1, None)
        .expect("cannot create feature");
    let (state, objective) = { (feature.state.as_ref().unwrap(), feature.objective.as_ref().unwrap()) };

    state.accept_solution_state(&mut insertion_ctx.solution);
    let fitness = objective.fitness(&insertion_ctx);
    let estimation = objective.estimate(&MoveContext::Route {
        solution_ctx: &insertion_ctx.solution,
        route_ctx: &insertion_ctx.solution.routes[route_idx],
        job: &get_job_by_id(&insertion_ctx, job_id).unwrap(),
    });

    assert_eq!(estimation, expected_estimation);
    assert_eq!(fitness, expected_fitness);
}

#[test]
fn can_return_err_if_feature_cannot_be_created() {
    let result = create_tour_compactness_feature("compact", create_empty_problem().jobs.clone(), 0, 1, None);

    assert!(result.is_err());
}
