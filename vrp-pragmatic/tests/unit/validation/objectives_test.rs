use super::*;
use crate::format::problem::Objective::*;
use crate::helpers::create_empty_problem;
use crate::helpers::*;

#[test]
fn can_fallback_to_default() {
    let problem = Problem { objectives: None, ..create_empty_problem() };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);

    let result = validate_objectives(&ctx);

    assert!(result.is_ok());
}

parameterized_test! {can_detect_empty_objective, (objectives, expected), {
    can_detect_empty_objective_impl(objectives, expected);
}}

can_detect_empty_objective! {
    case01: (Some(vec![]), Some(())),
    case03: (Some(vec![MinimizeCost]), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Vec<Objective>>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1600_empty_objective(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1600".to_string()));
}

parameterized_test! {can_detect_duplicates, (objectives, expected), {
    can_detect_duplicates_impl(objectives, expected);
}}

can_detect_duplicates! {
    case01: (Some(vec![MinimizeCost]), None),
    case02: (Some(vec![MinimizeCost, MinimizeCost]), Some("minimize-cost".to_owned())),
    case03: (Some(vec![
                MinimizeCost,
                BalanceDistance,
                MultiObjective {
                    strategy: MultiStrategy::Sum,
                    objectives: vec![MinimizeCost, BalanceDistance],}
            ]),
        Some("balance-distance,minimize-cost".to_owned())),
}

fn can_detect_duplicates_impl(objectives: Option<Vec<Objective>>, expected: Option<String>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1601_duplicate_objectives(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1601".to_string()));
}

parameterized_test! {can_detect_missing_cost_objective, (objectives, expected), {
    can_detect_missing_cost_objective_impl(objectives, expected);
}}

can_detect_missing_cost_objective! {
    case01: (Some(vec![MinimizeCost]), None),
    case02: (Some(vec![MinimizeDuration]), None),
    case03: (Some(vec![MinimizeDistance]), None),
    case04: (Some(vec![BalanceDistance]), Some(())),
}

fn can_detect_missing_cost_objective_impl(objectives: Option<Vec<Objective>>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1602_no_cost_objective(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1602".to_string()));
}

#[test]
fn can_detect_missing_value_jobs() {
    let problem = Problem {
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, MaximizeValue { breaks: None }, MinimizeCost]),
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1603_no_jobs_with_value_objective(&ctx, &objectives);

    assert_eq!(result.err().unwrap().code, "E1603".to_string());
}

#[test]
fn can_detect_missing_order_jobs() {
    let problem = Problem {
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, TourOrder, MinimizeCost]),
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1604_no_jobs_with_order_objective(&ctx, &objectives);

    assert_eq!(result.err().unwrap().code, "E1604".to_string());
}

parameterized_test! {can_detect_invalid_value_or_order, (value, order, expected), {
    can_detect_invalid_value_or_order_impl(value, order, expected);
}}

can_detect_invalid_value_or_order! {
    case01: (Some(0.), Some(1), Some("E1605".to_string())),
    case02: (Some(1.), Some(1), None),
    case03: (Some(0.), None, Some("E1605".to_string())),
    case04: (None, Some(0), Some("E1605".to_string())),
}

fn can_detect_invalid_value_or_order_impl(value: Option<f64>, order: Option<i32>, expected: Option<String>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                deliveries: Some(vec![JobTask { order, ..create_task((1., 0.), None) }]),
                value,
                ..create_job("job1")
            }],
            ..create_empty_plan()
        },
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);

    let result = check_e1605_check_positive_value_and_order(&ctx);

    assert_eq!(result.err().map(|e| e.code), expected);
}

parameterized_test! {can_detect_multiple_cost_objective, (objectives, expected), {
    can_detect_multiple_cost_objective_impl(objectives, expected);
}}

can_detect_multiple_cost_objective! {
    case01: (Some(vec![MinimizeCost]), None),
    case02: (Some(vec![MinimizeCost, MinimizeCost]), Some(())),
    case03: (Some(vec![MinimizeCost, MinimizeDuration]), Some(())),
    case04: (Some(vec![MinimizeCost, MinimizeDistance]), Some(())),
    case05: (Some(vec![MinimizeDuration, MinimizeDistance]), Some(())),
}

fn can_detect_multiple_cost_objective_impl(objectives: Option<Vec<Objective>>, expected: Option<()>) {
    let problem = Problem { objectives, ..create_empty_problem() };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1606_check_multiple_cost_objectives(&objectives);

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1606".to_string()));
}

parameterized_test! {can_detect_missing_value_objective, (objectives, expected), {
    can_detect_missing_value_objective_impl(objectives, expected);
}}

can_detect_missing_value_objective! {
    case01: (Some(vec![
                MinimizeUnassigned { breaks: None },
                MinimizeCost,
            ]), Some("E1607".to_string())),
    case02: (Some(vec![
                MinimizeUnassigned { breaks: None },
                MaximizeValue { breaks: None },
                MinimizeCost,
            ]), None),
    case03: (None, None),
}

fn can_detect_missing_value_objective_impl(objectives: Option<Vec<Objective>>, expected: Option<String>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                deliveries: Some(vec![create_task((1., 0.), None)]),
                value: Some(1.),
                ..create_job("job1")
            }],
            ..create_empty_plan()
        },
        objectives,
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap_or_default();

    let result = check_e1607_jobs_with_value_but_no_objective(&ctx, objectives.as_slice());

    assert_eq!(result.err().map(|e| e.code), expected);
}
