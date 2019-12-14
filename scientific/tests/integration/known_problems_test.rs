use crate::helpers::*;
use core::construction::states::InsertionContext;
use core::models::Problem;
use core::refinement::objectives::{Objective, PenalizeUnassigned};
use core::refinement::recreate::{Recreate, RecreateWithCheapest};
use core::refinement::RefinementContext;
use core::utils::DefaultRandom;
use std::sync::Arc;

parameterized_test! {can_solve_problem_with_cheapest_insertion_heuristic, (problem, expected, cost), {
    can_solve_problem_with_cheapest_insertion_heuristic_impl(Arc::new(problem), expected, cost);
}}

can_solve_problem_with_cheapest_insertion_heuristic! {
    case1: (
        create_c101_25_problem(),
        vec![
            vec!["13", "17", "18", "19", "15"],
            vec!["20", "24", "25", "10", "11", "9", "6", "23", "22", "21"],
            vec!["5", "3", "7", "8", "16", "14", "12", "4", "2", "1"],
        ],
        259.15),
    case2: (
        create_c101_100_problem(),
        vec![
             vec!["13", "17", "18", "19", "15", "16", "14", "12", "99"],
             vec!["20", "24", "25", "27", "29", "30", "28", "26", "23", "22", "21", "47"],
             vec!["32", "33", "31", "35", "37", "38","39", "36", "34"],
             vec!["43", "42", "41", "40", "44", "46", "45", "48", "51", "50", "52", "49"],
             vec!["5", "3", "7", "8", "10", "11", "9", "6", "4", "2", "1", "75"],
             vec!["57", "55", "54", "53", "56", "58", "60", "59"],
             vec!["67", "65", "63", "62", "74", "72", "61", "64", "68", "66", "69"],
             vec!["81", "78", "76", "71", "70", "73", "77", "79", "80"],
             vec!["90", "87", "86", "83", "82", "84", "85", "88", "89", "91"],
             vec!["98", "96", "95", "94", "92", "93", "97", "100"]
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
    let insertion_ctx = RecreateWithCheapest::default().run(
        &RefinementContext::new(problem.clone(), false, 2),
        InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())),
    );

    let result_cost = PenalizeUnassigned::new(1000.).estimate(&insertion_ctx);
    assert_eq!(get_customer_ids_from_routes_sorted(&insertion_ctx), expected);
    assert_eq!(result_cost.actual.round(), cost.round());
    assert_eq!(result_cost.penalty, 0.0);
}
