use super::*;
use crate::construction::heuristics::{InsertionContext, UnassignmentInfo};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_solution_context};
use crate::helpers::models::problem::{get_job_id, test_single_with_id};
use crate::helpers::models::solution::*;
use crate::models::common::ValueDimension;
use crate::models::problem::Single;

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
    let state_key = 1;
    let (constraint, _) = TotalValue::maximize(
        1000.,
        0.1,
        Arc::new(|_| 0.),
        ValueFn::Left(Arc::new(move |_| value)),
        Arc::new(|job, _| job),
        state_key,
        1,
    );
    let constraint = create_constraint_pipeline_with_module(constraint);
    let mut route_ctx = create_empty_route_ctx();
    route_ctx.state_mut().put_route_state(state_key, max_cost);
    let solution_ctx = create_empty_solution_context();

    let result = constraint.evaluate_soft_route(&solution_ctx, &route_ctx, &Job::Single(test_single_with_id("job")));

    assert_eq!(result, expected);
}

#[test]
fn can_estimate_solution_value() {
    let state_key = 1;
    let (_, objective) = TotalValue::maximize(
        1000.,
        0.1,
        Arc::new(|solution_ctx| solution_ctx.unassigned.len() as f64 * -100.),
        ValueFn::Left(Arc::new(move |_| 0.)),
        Arc::new(|job, _| job),
        state_key,
        1,
    );
    let mut solution = create_empty_solution_context();
    solution.unassigned.insert(Job::Single(test_single_with_id("job1")), UnassignmentInfo::Simple(0));
    solution.unassigned.insert(Job::Single(test_single_with_id("job2")), UnassignmentInfo::Simple(1));
    let insertion_ctx = InsertionContext { solution, ..create_empty_insertion_context() };

    let fitness = objective.fitness(&insertion_ctx);

    assert_eq!(fitness, 200.);
}

#[test]
fn can_merge_value() {
    let state_key = 1;
    let (constraint, _) = TotalValue::maximize(
        1000.,
        0.1,
        Arc::new(|solution_ctx| solution_ctx.unassigned.len() as f64 * -100.),
        ValueFn::Left(Arc::new(move |job| match get_job_id(job).as_str() {
            "source" => 10.,
            "candidate" => 2.,
            _ => unreachable!(),
        })),
        Arc::new(|job, value| {
            let single = job.to_single();
            let mut dimens = single.dimens.clone();
            dimens.set_value("value", value);

            Job::Single(Arc::new(Single { places: single.places.clone(), dimens }))
        }),
        state_key,
        1,
    );
    let source = Job::Single(test_single_with_id("source"));
    let candidate = Job::Single(test_single_with_id("candidate"));

    let merged = constraint.merge(source, candidate).unwrap();

    assert_eq!(merged.dimens().get_value::<f64>("value").cloned(), Some(12.))
}
