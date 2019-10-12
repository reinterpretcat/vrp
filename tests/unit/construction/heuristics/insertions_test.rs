use crate::construction::heuristics::insertions::create_cheapest_insertion_heuristic;
use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::get_customer_ids_from_routes_sorted;
use crate::helpers::streams::input::*;
use crate::models::{Extras, Problem};
use crate::objectives::{ObjectiveFunction, PenalizeUnassigned};
use std::io::BufWriter;
use std::sync::Arc;

parameterized_test! {can_solve_solomon_problem_with_cheapest_insertion_heuristic, (problem, expected, cost), {
    can_solve_solomon_problem_with_cheapest_insertion_heuristic_impl(problem, expected, cost);
}}

can_solve_solomon_problem_with_cheapest_insertion_heuristic! {
    case1: (
        Arc::new(create_c101_25_problem()),
        vec![
            vec!["c13", "c17", "c18", "c19", "c15"],
            vec!["c20", "c24", "c25", "c10", "c11", "c9", "c6", "c23", "c22", "c21"],
            vec!["c5", "c3", "c7", "c8", "c16", "c14", "c12", "c4", "c2", "c1"],
        ],
        259.15),
    case2: (
        Arc::new(create_c101_100_problem()),
        vec![
             vec!["c13", "c17", "c18", "c19", "c15", "c16", "c14", "c12", "c99"],
             vec!["c20", "c24", "c25", "c27", "c29", "c30", "c28", "c26", "c23", "c22", "c21", "c47"],
             vec!["c32", "c33", "c31", "c35", "c37", "c38","c39", "c36", "c34"],
             vec!["c43", "c42", "c41", "c40", "c44", "c46", "c45", "c48", "c51", "c50", "c52", "c49"],
             vec!["c5", "c3", "c7", "c8", "c10", "c11", "c9", "c6", "c4", "c2", "c1", "c75"],
             vec!["c57", "c55", "c54", "c53", "c56", "c58", "c60", "c59"],
             vec!["c67", "c65", "c63", "c62", "c74", "c72", "c61", "c64", "c68", "c66", "c69"],
             vec!["c81", "c78", "c76", "c71", "c70", "c73", "c77", "c79", "c80"],
             vec!["c90", "c87", "c86", "c83", "c82", "c84", "c85", "c88", "c89", "c91"],
             vec!["c98", "c96", "c95", "c94", "c92", "c93", "c97", "c100"]
        ],
        878.36),
}

fn can_solve_solomon_problem_with_cheapest_insertion_heuristic_impl(
    problem: Arc<Problem>,
    expected: Vec<Vec<&str>>,
    cost: f64,
) {
    let heuristic = create_cheapest_insertion_heuristic();

    let result = heuristic.process(InsertionContext::new(problem.clone()));

    let solution = result.solution.into_solution(Extras::default());
    assert_eq!(get_customer_ids_from_routes_sorted(&solution), expected);
    let result = PenalizeUnassigned::new(1000.).estimate(&problem, &solution);
    assert_eq!(result.actual.round(), cost.round());
    assert_eq!(result.penalty, 0.0);
}
