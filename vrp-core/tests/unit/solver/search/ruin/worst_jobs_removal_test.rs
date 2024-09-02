use super::{Ruin, WorstJobRemoval};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::solver::search::RemovalLimits;
use std::sync::Arc;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    case_01: ((5, 3), vec![4, 2, 0, 0, 0], vec!["c3", "c4", "c8", "c9"]),
    case_02: ((5, 3), vec![6, 2, 0, 0, 0], vec!["c14", "c2", "c3", "c4", "c8", "c9"]),
}

fn can_ruin_solution_with_matrix_routes_impl(matrix: (usize, usize), ints: Vec<i32>, expected_ids: Vec<&str>) {
    let reals = vec![];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, 1000., false);
    let limits = RemovalLimits { removed_activities_range: 10..10, affected_routes_range: 2..2 };
    let insertion_ctx: InsertionContext = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
    );

    let insertion_ctx = WorstJobRemoval::new(4, limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}
