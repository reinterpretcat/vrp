use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::get_customer_ids_from_routes_sorted;
use crate::helpers::refinement::create_with_cheapest;
use crate::helpers::streams::input::*;
use crate::models::{Extras, Problem};
use crate::objectives::{ObjectiveFunction, PenalizeUnassigned};
use crate::refinement::recreate::{Recreate, RecreateWithCheapest};
use crate::utils::DefaultRandom;
use std::io::BufWriter;
use std::sync::Arc;

parameterized_test! {can_solve_problem_with_cheapest_insertion_heuristic, (problem, expected, cost), {
    can_solve_problem_with_cheapest_insertion_heuristic_impl(Arc::new(problem), expected, cost);
}}

can_solve_problem_with_cheapest_insertion_heuristic! {
    case1: (
        create_c101_25_problem(),
        vec![
            vec!["c13", "c17", "c18", "c19", "c15"],
            vec!["c20", "c24", "c25", "c10", "c11", "c9", "c6", "c23", "c22", "c21"],
            vec!["c5", "c3", "c7", "c8", "c16", "c14", "c12", "c4", "c2", "c1"],
        ],
        259.15),
    case2: (
        create_c101_100_problem(),
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
    case3: (
        create_lc101_problem(),
        vec![
            vec!["mlt1", "mlt0", "mlt1", "mlt3", "mlt3", "mlt5", "mlt4", "mlt2", "mlt4", "mlt2", "mlt5", "mlt0"],
            vec!["mlt10", "mlt10", "mlt12", "mlt12", "mlt14", "mlt15", "mlt13", "mlt14", "mlt11", "mlt11", "mlt13", "mlt15"],
            vec!["mlt16", "mlt17", "mlt16", "mlt18", "mlt17", "mlt20", "mlt18", "mlt19", "mlt19", "mlt20"],
            vec!["mlt22", "mlt21", "mlt22", "mlt21", "mlt23", "mlt23", "mlt24", "mlt24", "mlt27", "mlt27", "mlt26", "mlt26", "mlt25", "mlt25"],
            vec!["mlt31", "mlt31", "mlt29", "mlt28", "mlt30", "mlt28", "mlt29", "mlt30"],
            vec!["mlt37", "mlt35", "mlt33", "mlt32", "mlt33", "mlt35", "mlt37", "mlt34", "mlt34", "mlt32", "mlt36", "mlt36"],
            vec!["mlt42", "mlt40", "mlt40", "mlt39", "mlt38", "mlt42", "mlt39", "mlt38", "mlt41", "mlt41"],
            vec!["mlt47", "mlt46", "mlt45", "mlt46", "mlt43", "mlt44", "mlt43", "mlt47", "mlt44", "mlt45"],
            vec!["mlt51", "mlt49", "mlt51", "mlt49", "mlt48", "mlt48", "mlt50", "mlt50", "mlt52", "mlt52"],
            vec!["mlt6", "mlt6", "mlt8", "mlt9", "mlt9", "mlt7", "mlt7", "mlt8"]
        ],
        828.937),
}

fn can_solve_problem_with_cheapest_insertion_heuristic_impl(
    problem: Arc<Problem>,
    expected: Vec<Vec<&str>>,
    cost: f64,
) {
    let result = create_with_cheapest(problem.clone(), Arc::new(DefaultRandom::new()));

    let solution = result.solution.into_solution(Arc::new(Extras::default()));
    assert_eq!(get_customer_ids_from_routes_sorted(&solution), expected);
    let result = PenalizeUnassigned::new(1000.).estimate(&problem, &solution);
    assert_eq!(result.actual.round(), cost.round());
    assert_eq!(result.penalty, 0.0);
}
