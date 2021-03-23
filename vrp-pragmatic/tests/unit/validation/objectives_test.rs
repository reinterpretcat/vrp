use super::*;
use crate::format::problem::Objective::*;
use crate::helpers::create_empty_problem;

fn min_cost() -> Objective {
    MinimizeCost
}

fn balance_dist() -> Objective {
    BalanceDistance { options: None }
}

#[test]
fn can_fallback_to_default() {
    let problem = Problem { objectives: None, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);

    let result = validate_objectives(&ctx);

    assert!(result.is_ok());
}

parameterized_test! {can_detect_empty_objective, (objectives, expected), {
    can_detect_empty_objective_impl(objectives, expected);
}}

can_detect_empty_objective! {
    case01: (Some(vec![vec![]]), Some(())),
    case02: (Some(vec![]), Some(())),
    case03: (Some(vec![vec![min_cost()]]), None),
    case04: (Some(vec![vec![], vec![min_cost() ]]), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1600_empty_objective(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1600".to_string()));
}

parameterized_test! {can_detect_duplicates, (objectives, expected), {
    can_detect_duplicates_impl(objectives, expected);
}}

can_detect_duplicates! {
    case01: (Some(vec![vec![min_cost()]]), None),
    case02: (Some(vec![vec![min_cost()], vec![min_cost() ]]), Some("minimize-cost".to_owned())),
    case03: (Some(vec![
                vec![min_cost(), balance_dist(), balance_dist()],
                vec![min_cost()]
            ]),
        Some("balance-distance,minimize-cost".to_owned())),
}

fn can_detect_duplicates_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<String>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1601_duplicate_objectives(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1601".to_string()));
}

parameterized_test! {can_detect_missing_cost_objective, (objectives, expected), {
    can_detect_missing_cost_objective_impl(objectives, expected);
}}

can_detect_missing_cost_objective! {
    case01: (Some(vec![vec![min_cost()]]), None),
    case02: (Some(vec![vec![balance_dist()]]), Some(())),
    case03: (Some(vec![vec![], vec![balance_dist()]]), Some(())),
}

fn can_detect_missing_cost_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1602_no_cost_objective(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1602".to_string()));
}
