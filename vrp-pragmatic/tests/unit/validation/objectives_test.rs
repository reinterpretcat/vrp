use crate::helpers::create_empty_problem;
use crate::json::problem::Objective::*;
use crate::json::problem::*;
use crate::validation::{ValidationContext, VALIDATION_MESSAGE_PREFIX};

fn min_cost() -> Objective {
    MinimizeCost { goal: None }
}

fn balance_dist() -> Objective {
    BalanceDistance { threshold: None }
}

parameterized_test! {can_detect_empty_objective, (objectives, expected), {
    can_detect_empty_objective_impl(objectives, expected);
}}

can_detect_empty_objective! {
    case01: (None, None),
    case02: (Some(Objectives { primary: vec![], secondary: None }), Some(())),
    case03: (Some(Objectives { primary: vec![min_cost() ], secondary: None}), None),
    case04: (Some(Objectives { primary: vec![], secondary: Some(vec![]) }), Some(())),
    case05: (Some(Objectives { primary: vec![], secondary: Some(vec![min_cost() ]) }), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Objectives>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };

    let result = ValidationContext::new(&problem, None).validate();

    assert_eq!(
        result.err(),
        expected.map(|_| format!("{}E1009: An empty objective specified", VALIDATION_MESSAGE_PREFIX))
    );
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

    let result = ValidationContext::new(&problem, None).validate();

    assert_eq!(
        result.err(),
        expected.map(|names| format!("{}E1010: Duplicate objective specified: {}", VALIDATION_MESSAGE_PREFIX, names))
    );
}
