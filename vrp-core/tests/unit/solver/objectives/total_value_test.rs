use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_solution_context};
use crate::helpers::models::problem::test_single_with_id;
use crate::helpers::models::solution::*;

parameterized_test! {can_estimate_job_value, (value, max_cost, expected), {
    can_estimate_job_value_impl(value, max_cost, expected);
}}

can_estimate_job_value! {
    case_01: (100., 1000., -10.),
    case_02: (50., 1000., -5.),
    case_03: (50., 100., -0.5),
    case_04: (100., 0., -10.),
    case_05: (50., 0., -5.),
}

fn can_estimate_job_value_impl(value: f64, max_cost: f64, expected: f64) {
    let (constraint, _) =
        TotalValue::maximize(1000., 0.1, Arc::new(|_| 0.), Arc::new(move |_| value), Arc::new(|job, _| job));
    let constraint = create_constraint_pipeline_with_module(constraint);
    let mut route_ctx = create_empty_route_ctx();
    route_ctx.state_mut().put_route_state(TOTAL_VALUE_KEY, max_cost);
    let solution_ctx = create_empty_solution_context();

    let result = constraint.evaluate_soft_route(&solution_ctx, &route_ctx, &Job::Single(test_single_with_id("job")));

    assert_eq!(result, expected);
}

#[test]
fn can_estimate_solution_value() {
    let (_, objective) = TotalValue::maximize(
        1000.,
        0.1,
        Arc::new(|solution_ctx| solution_ctx.unassigned.len() as f64 * -100.),
        Arc::new(move |_| 0.),
        Arc::new(|job, _| job),
    );
    let mut solution = create_empty_solution_context();
    solution.unassigned.insert(Job::Single(test_single_with_id("job1")), 0);
    solution.unassigned.insert(Job::Single(test_single_with_id("job2")), 1);
    let insertion_ctx = InsertionContext { solution, ..create_empty_insertion_context() };

    let fitness = objective.fitness(&insertion_ctx);

    assert_eq!(fitness, 200.);
}
