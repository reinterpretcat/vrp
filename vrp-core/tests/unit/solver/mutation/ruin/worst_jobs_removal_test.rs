use std::sync::Arc;

use super::{Ruin, WorstJobRemoval};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes};
use crate::helpers::utils::random::FakeRandom;
use crate::solver::mutation::JobRemovalLimit;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    case_01: ((5, 3), vec![32, 0, 2, 0, 2, 0, 2], vec!["c14", "c3", "c4", "c9"]),
    case_02: ((5, 3), vec![32, 0, 3, 0, 3, 0, 3], vec!["c13", "c14", "c3", "c4", "c8", "c9"]),
}

fn can_ruin_solution_with_matrix_routes_impl(matrix: (usize, usize), ints: Vec<i32>, expected_ids: Vec<&str>) {
    let reals = vec![];

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1, |data| (data.clone(), data));
    let insertion_ctx: InsertionContext = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        Arc::new(FakeRandom::new(ints, reals)),
    );

    let insertion_ctx = WorstJobRemoval::new(4, JobRemovalLimit::new(1, 32, 1.))
        .run(&mut create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}
