use super::*;
use crate::helpers::create_empty_problem;
use crate::json::problem::Objective::*;

fn min_cost() -> Objective {
    MinimizeCost { goal: None }
}

fn balance_dist() -> Objective {
    BalanceDistance { threshold: None, tolerance: None }
}

#[test]
fn can_fallback_to_default() {
    let problem = Problem { objectives: None, ..create_empty_problem() };

    let result = ValidationContext::new(&problem, None).validate();

    assert_eq!(result.err(), None);
}

parameterized_test! {can_detect_empty_objective, (objectives, expected), {
    can_detect_empty_objective_impl(objectives, expected);
}}

can_detect_empty_objective! {
    case01: (Some(Objectives { primary: vec![], secondary: None }), Some(())),
    case02: (Some(Objectives { primary: vec![min_cost() ], secondary: None}), None),
    case03: (Some(Objectives { primary: vec![], secondary: Some(vec![]) }), Some(())),
    case04: (Some(Objectives { primary: vec![], secondary: Some(vec![min_cost() ]) }), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Objectives>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1009_empty_objective(&objectives);

    assert_eq!(result.err(), expected.map(|_| "E1009: An empty objective specified".to_string()));
}

parameterized_test! {can_detect_duplicates, (objectives, expected), {
    can_detect_duplicates_impl(objectives, expected);
}}

can_detect_duplicates! {
    case01: (Some(Objectives { primary: vec![min_cost() ], secondary: None}), None),
    case02: (Some(Objectives { primary: vec![], secondary: Some(vec![min_cost() ]) }), None),
    case03: (Some(Objectives { primary: vec![min_cost()], secondary: Some(vec![min_cost() ]) }), Some("minimize-cost".to_owned())),
    case04: (Some(Objectives {
            primary: vec![min_cost()],
            secondary: Some(vec![min_cost() ]) }),
        Some("minimize-cost".to_owned())),
    case05: (Some(Objectives {
            primary: vec![min_cost(), balance_dist(), balance_dist()],
            secondary: Some(vec![min_cost() ]) }),
        Some("balance-distance,minimize-cost".to_owned())),
}

fn can_detect_duplicates_impl(objectives: Option<Objectives>, expected: Option<String>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1010_duplicate_objectives(&objectives);

    assert_eq!(result.err(), expected.map(|names| format!("E1010: Duplicate objective specified: {}", names)));
}

parameterized_test! {can_detect_missing_cost_objective, (objectives, expected), {
    can_detect_missing_cost_objective_impl(objectives, expected);
}}

can_detect_missing_cost_objective! {
    case01: (Some(Objectives { primary: vec![min_cost() ], secondary: None}), None),
    case02: (Some(Objectives { primary: vec![], secondary: Some(vec![min_cost() ]) }), None),
    case03: (Some(Objectives { primary: vec![balance_dist()], secondary: None }), Some(())),
    case04: (Some(Objectives { primary: vec![], secondary: Some(vec![balance_dist() ]) }), Some(())),
}

fn can_detect_missing_cost_objective_impl(objectives: Option<Objectives>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1011_no_cost_value_objective(&objectives);

    assert_eq!(result.err(), expected.map(|_| "E1011: Missing cost objective".to_string()));
}
