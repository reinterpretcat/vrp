/* Uses predefined values to control algorithm execution.
int distribution values:
1. route index in solution
2*. job index in selected route tour
3*. selected algorithm: 1: sequential algorithm(**)
4*. string removal index(-ies)
double distribution values:
1. string count
2*. string size(-s)
(*) - specific for each route.
(**) - calls more int and double distributions:
    int 5. split start
    dbl 3. alpha param
*/

use super::{AdjustedStringRemoval, Ruin};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use std::sync::Arc;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, reals, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, reals, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    case_01_sequential: ((10, 1), vec![0, 3, 1, 2], vec![1., 5.], vec!["c1", "c2", "c3", "c4", "c5"]),
    case_02_preserved: ((10, 1), vec![0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.005], vec!["c0", "c1", "c2", "c5", "c6"]),
    case_03_preserved: ((10, 1), vec![0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c0", "c1", "c2", "c6", "c7"]),
    case_04_preserved: ((10, 1), vec![0, 2, 2, 3, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c2", "c6", "c7", "c8", "c9"]),
    case_05_sequential: ((5, 3), vec![1, 2, 1, 2], vec![1., 3.], vec!["c6", "c7", "c8"]),
    case_06_sequential: ((5, 3), vec![0, 2, 1, 2, 1, 3, 2], vec![2., 3., 2.], vec!["c1", "c2", "c3", "c7", "c8"]),
    case_07_sequential: ((5, 3), vec![1, 1, 1, 2, 1, 2, 1, 2, 1, 2], vec![3., 3., 3., 3.], vec!["c1", "c11", "c12", "c13", "c2", "c3", "c6", "c7", "c8"]),
    case_08_preserved: ((5, 3), vec![1, 1, 2, 1, 3], vec![1., 3., 0.5], vec!["c5", "c6", "c9"]),
    case_09_preserved: ((5, 3), vec![1, 3, 2, 1, 3], vec![1., 3., 0.5], vec!["c5", "c6", "c7"]),
}

fn can_ruin_solution_with_matrix_routes_impl(
    matrix: (usize, usize),
    ints: Vec<i32>,
    reals: Vec<f64>,
    expected_ids: Vec<&str>,
) {
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, false);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
    );

    let insertion_ctx = AdjustedStringRemoval::default()
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}
