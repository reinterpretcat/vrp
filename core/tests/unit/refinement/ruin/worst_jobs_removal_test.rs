use std::sync::Arc;

use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::utils::random::FakeRandom;
use crate::refinement::ruin::{Ruin, WorstJobRemoval};
use crate::refinement::RefinementContext;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    case_01: ((5, 3), vec![0, 2, 0, 2, 0, 2], vec!["c14", "c3", "c4", "c9"]),
    case_02: ((5, 3), vec![0, 3, 0, 3, 0, 3], vec!["c13", "c14", "c3", "c4", "c8", "c9"]),
}

fn can_ruin_solution_with_matrix_routes_impl(matrix: (usize, usize), ints: Vec<i32>, expected_ids: Vec<&str>) {
    let reals = vec![];

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let insertion_ctx: InsertionContext = InsertionContext::new_from_solution(
        Arc::new(problem),
        (Arc::new(solution), None),
        Arc::new(FakeRandom::new(ints, reals)),
    );

    let insertion_ctx =
        WorstJobRemoval::default().run(&RefinementContext::new(insertion_ctx.problem.clone(), false, 1), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}
