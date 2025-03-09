use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::Problem;
use vrp_core::rosomaxa::evolution::TelemetryMode;
use vrp_core::solver::RefinementContext;
use vrp_core::solver::create_elitism_population;
use vrp_core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_core::utils::{Environment, Float};

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
        create_lc101_problem(),
        vec![
            vec!["1", "0", "1", "3", "3", "5", "4", "2", "4", "2", "5", "0"],
            vec!["10", "10", "12", "12", "14", "15", "13", "14", "11", "11", "13", "15"],
            vec!["16", "17", "16", "18", "17", "20", "18", "19", "19", "20"],
            vec!["22", "21", "22", "21", "23", "23", "24", "24", "27", "27", "26", "26", "25", "25"],
            vec!["31", "31", "29", "28", "30", "28", "29", "30"],
            vec!["37", "35", "33", "32", "33", "35", "37", "34", "34", "32", "36", "36"],
            vec!["42", "40", "40", "39", "38", "42", "39", "38", "41", "41"],
            vec!["47", "46", "45", "46", "43", "44", "43", "47", "44", "45"],
            vec!["51", "49", "51", "49", "48", "48", "50", "50", "52", "52"],
            vec!["6", "6", "8", "9", "9", "7", "7", "8"]
        ],
        828.937),
}

fn can_solve_problem_with_cheapest_insertion_heuristic_impl(
    problem: Arc<Problem>,
    expected: Vec<Vec<&str>>,
    cost: Float,
) {
    let environment = Arc::new(Environment::default());
    let refinement_ctx = RefinementContext::new(
        problem.clone(),
        Box::new(create_elitism_population(problem.goal.clone(), environment.clone())),
        TelemetryMode::None,
        environment.clone(),
    );
    let insertion_ctx = RecreateWithCheapest::new(environment.random.clone())
        .run(&refinement_ctx, InsertionContext::new(problem, environment));

    let result_cost = insertion_ctx.get_total_cost().unwrap_or_default();
    assert_eq!(result_cost.round(), cost.round());
    assert_eq!(get_customer_ids_from_routes_sorted(&insertion_ctx), expected);
}
