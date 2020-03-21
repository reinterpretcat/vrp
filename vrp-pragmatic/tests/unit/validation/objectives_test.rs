use crate::helpers::create_empty_problem;
use crate::json::problem::Objective::*;
use crate::json::problem::*;
use crate::validation::{ValidationContext, VALIDATION_MESSAGE_PREFIX};

parameterized_test! {can_detect_empty_objective, (objectives, expected), {
    can_detect_empty_objective_impl(objectives, expected);
}}

can_detect_empty_objective! {
    case01: (None, None),
    case02: (Some(Objectives { primary: vec![], secondary: None }), Some(())),
    case03: (Some(Objectives { primary: vec![MinimizeCost { goal: None } ], secondary: None}), None),
    case04: (Some(Objectives { primary: vec![], secondary: Some(vec![]) }), Some(())),
    case05: (Some(Objectives { primary: vec![], secondary: Some(vec![MinimizeCost { goal: None } ]) }), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Objectives>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };

    let result = ValidationContext::new(&problem, None).validate();

    assert_eq!(
        result.err(),
        expected.map(|_| format!("{}E1009: An empty objective specified", VALIDATION_MESSAGE_PREFIX))
    );
}
