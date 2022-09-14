use super::*;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::{get_job_id, test_single_with_id};
use crate::helpers::models::solution::*;
use crate::models::common::ValueDimension;
use crate::models::problem::Single;

const VIOLATION_CODE: ViolationCode = 1;

parameterized_test! {can_estimate_job_value, (value, expected), {
    can_estimate_job_value_impl(value, expected);
}}

can_estimate_job_value! {
    case_01: (100., -100.),
    case_02: (1., -1.),
    case_03: (0., 0.),
}

fn can_estimate_job_value_impl(value: f64, expected: f64) {
    let objective = maximize_total_job_value(
        JobReadValueFn::Left(Arc::new(move |_| value)),
        Arc::new(|job, _| job),
        VIOLATION_CODE,
    )
    .unwrap()
    .objective
    .unwrap();
    let route_ctx = create_empty_route_ctx();
    let solution_ctx = create_empty_solution_context();

    let result =
        objective.estimate(&MoveContext::route(&solution_ctx, &route_ctx, &Job::Single(test_single_with_id("job"))));

    assert_eq!(result, expected);
}

#[test]
fn can_merge_value() {
    let constraint = maximize_total_job_value(
        JobReadValueFn::Left(Arc::new(move |job| match get_job_id(job).as_str() {
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
        VIOLATION_CODE,
    )
    .unwrap()
    .constraint
    .unwrap();
    let source = Job::Single(test_single_with_id("source"));
    let candidate = Job::Single(test_single_with_id("candidate"));

    let merged = constraint.merge(source, candidate).unwrap();

    assert_eq!(merged.dimens().get_value::<f64>("value").cloned(), Some(12.))
}
