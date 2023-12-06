use super::*;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::{get_job_id, SingleBuilder};
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
    let objective = create_maximize_total_job_value_feature(
        "value",
        JobReadValueFn::Left(Arc::new(move |_| value)),
        Arc::new(|job, _| job),
        VIOLATION_CODE,
    )
    .unwrap()
    .objective
    .unwrap();
    let route_ctx = RouteContextBuilder::default().build();
    let solution_ctx = create_empty_solution_context();

    let result = objective.estimate(&MoveContext::route(
        &solution_ctx,
        &route_ctx,
        &SingleBuilder::default().id("job").build_as_job_ref(),
    ));

    assert_eq!(result, expected);
}

#[test]
fn can_merge_value() {
    let constraint = create_maximize_total_job_value_feature(
        "value",
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
    let source = SingleBuilder::default().id("source").build_as_job_ref();
    let candidate = SingleBuilder::default().id("candidate").build_as_job_ref();

    let merged = constraint.merge(source, candidate).unwrap();

    assert_eq!(merged.dimens().get_value::<f64>("value").cloned(), Some(12.))
}
