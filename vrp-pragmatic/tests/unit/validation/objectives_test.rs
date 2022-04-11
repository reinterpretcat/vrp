use super::*;
use crate::format::problem::Objective::*;
use crate::helpers::create_empty_problem;
use crate::helpers::*;

fn min_cost() -> Objective {
    MinimizeCost
}

fn balance_dist() -> Objective {
    BalanceDistance { options: None }
}

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
    case01: (Some(vec![vec![]]), Some(())),
    case02: (Some(vec![]), Some(())),
    case03: (Some(vec![vec![min_cost()]]), None),
    case04: (Some(vec![vec![], vec![min_cost() ]]), None),
}

fn can_detect_empty_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<()>) {
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
    case01: (Some(vec![vec![min_cost()]]), None),
    case02: (Some(vec![vec![balance_dist()]]), Some(())),
    case03: (Some(vec![vec![], vec![balance_dist()]]), Some(())),
}

fn can_detect_missing_cost_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<()>) {
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
        objectives: Some(vec![
            vec![MinimizeUnassignedJobs { breaks: None }],
            vec![MaximizeValue { reduction_factor: None, breaks: None }],
            vec![MinimizeCost],
        ]),
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
        objectives: Some(vec![
            vec![MinimizeUnassignedJobs { breaks: None }],
            vec![TourOrder { is_constrained: false }],
            vec![MinimizeCost],
        ]),
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

parameterized_test! {can_detect_missing_order_objective, (objectives, expected), {
    can_detect_missing_order_objective_impl(objectives, expected);
}}

can_detect_missing_order_objective! {
    case01: (Some(vec![
                vec![MinimizeUnassignedJobs { breaks: None }],
                vec![MinimizeCost],
            ]), Some("E1606".to_string())),
    case02: (Some(vec![
                vec![MinimizeUnassignedJobs { breaks: None }],
                vec![TourOrder { is_constrained: true }],
                vec![MinimizeCost],
            ]), None),
    case03: (None, None),
}

fn can_detect_missing_order_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<String>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                deliveries: Some(vec![JobTask { order: Some(1), ..create_task((1., 0.), None) }]),
                ..create_job("job1")
            }],
            ..create_empty_plan()
        },
        objectives,
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap_or_else(Vec::new);

    let result = check_e1606_jobs_with_order_but_no_objective(&ctx, objectives.as_slice());

    assert_eq!(result.err().map(|e| e.code), expected);
}

parameterized_test! {can_detect_missing_value_objective, (objectives, expected), {
    can_detect_missing_value_objective_impl(objectives, expected);
}}

can_detect_missing_value_objective! {
    case01: (Some(vec![
                vec![MinimizeUnassignedJobs { breaks: None }],
                vec![MinimizeCost],
            ]), Some("E1607".to_string())),
    case02: (Some(vec![
                vec![MinimizeUnassignedJobs { breaks: None }],
                vec![MaximizeValue { breaks: None, reduction_factor: None }],
                vec![MinimizeCost],
            ]), None),
    case03: (None, None),
}

fn can_detect_missing_value_objective_impl(objectives: Option<Vec<Vec<Objective>>>, expected: Option<String>) {
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
    let objectives = get_objectives(&ctx).unwrap_or_else(Vec::new);

    let result = check_e1607_jobs_with_value_but_no_objective(&ctx, objectives.as_slice());

    assert_eq!(result.err().map(|e| e.code), expected);
}

#[test]
fn can_detect_missing_area_objective() {
    let problem = Problem {
        plan: Plan { areas: Some(vec![Area { id: "area1".to_string(), jobs: vec![] }]), ..create_empty_plan() },
        objectives: Some(vec![vec![MinimizeUnassignedJobs { breaks: None }], vec![MinimizeCost]]),
        ..create_empty_problem()
    };
    let coord_index = CoordIndex::new(&problem);
    let ctx = ValidationContext::new(&problem, None, &coord_index);
    let objectives = get_objectives(&ctx).unwrap();

    let result = check_e1608_areas_but_no_objective(&ctx, &objectives);

    assert_eq!(result.err().unwrap().code, "E1608".to_string());
}
