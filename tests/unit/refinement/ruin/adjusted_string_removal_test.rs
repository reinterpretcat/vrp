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

use crate::construction::heuristics::create_cheapest_insertion_heuristic;
use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::{get_customer_ids_from_routes_sorted, get_sorted_customer_ids_from_jobs};
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::streams::input::LilimBuilder;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::ObjectiveCost;
use crate::refinement::ruin::{AdjustedStringRemoval, RuinStrategy};
use crate::refinement::RefinementContext;
use crate::streams::input::text::LilimProblem;
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
    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let refinement_ctx = RefinementContext {
        problem: Arc::new(problem),
        locked: Default::default(),
        population: vec![(Arc::new(solution), ObjectiveCost::new(0., 0.))],
        random: Arc::new(FakeRandom::new(ints, reals)),
        generation: 0,
    };

    let insertion_ctx = AdjustedStringRemoval::default().ruin_solution(&refinement_ctx).unwrap();

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}

#[test]
fn can_ruin_solution_with_multi_jobs() {
    let ints = vec![0, 3, 1, 2];
    let reals = vec![1., 3.];
    let expected_remove_ids = vec!["mlt2", "mlt3"];
    let expected_route_ids = vec![vec!["mlt1", "mlt1", "mlt0", "mlt0"]];

    let problem = Arc::new(
        LilimBuilder::new()
            .set_vehicle((1, 200))
            .add_customer((0, 0, 0, 0, 0, 1000, 0, 0, 0))
            .add_customer((1, 1, 0, -1, 0, 1000, 0, 2, 0))
            .add_customer((2, 2, 0, 1, 0, 1000, 0, 0, 1))
            .add_customer((3, 3, 0, -1, 0, 1000, 0, 4, 0))
            .add_customer((4, 4, 0, 1, 0, 1000, 0, 0, 3))
            .add_customer((5, 5, 0, -1, 0, 1000, 0, 6, 0))
            .add_customer((6, 6, 0, 1, 0, 1000, 0, 0, 5))
            .add_customer((7, 7, 0, -1, 0, 1000, 0, 8, 0))
            .add_customer((8, 8, 0, 1, 0, 1000, 0, 0, 7))
            .build()
            .parse_lilim()
            .unwrap(),
    );
    let heuristic = create_cheapest_insertion_heuristic();
    let solution = heuristic.process(InsertionContext::new(problem.clone())).solution.into_solution(Default::default());
    let refinement_ctx = RefinementContext {
        problem,
        locked: Default::default(),
        population: vec![(Arc::new(solution), ObjectiveCost::new(0.0, 0.0))],
        random: Arc::new(FakeRandom::new(ints, reals)),
        generation: 0,
    };

    let insertion_ctx = AdjustedStringRemoval::default().ruin_solution(&refinement_ctx).unwrap();

    //
    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_remove_ids);
    assert_eq!(
        get_customer_ids_from_routes_sorted(&insertion_ctx.solution.into_solution(Default::default())),
        expected_route_ids
    );
}
