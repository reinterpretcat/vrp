use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{TestSingleBuilder, get_job_id};
use crate::helpers::models::solution::*;
use crate::models::problem::Single;

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

parameterized_test! {can_estimate_job_value, (value, expected), {
    can_estimate_job_value_impl(value, expected);
}}

can_estimate_job_value! {
    case_01: (100., -100.),
    case_02: (1., -1.),
    case_03: (0., 0.),
}

fn can_estimate_job_value_impl(value: Float, expected: Float) {
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
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;

    let result = objective.estimate(&MoveContext::route(
        &solution_ctx,
        &route_ctx,
        &TestSingleBuilder::default().id("job").build_as_job_ref(),
    ));

    assert_eq!(result, expected);
}

#[test]
fn can_merge_value() {
    struct ValueDimenKey;
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
            dimens.set_value::<ValueDimenKey, _>(value);

            Job::Single(Arc::new(Single { places: single.places.clone(), dimens }))
        }),
        VIOLATION_CODE,
    )
    .unwrap()
    .constraint
    .unwrap();
    let source = TestSingleBuilder::default().id("source").build_as_job_ref();
    let candidate = TestSingleBuilder::default().id("candidate").build_as_job_ref();

    let merged = constraint.merge(source, candidate).unwrap();

    assert_eq!(merged.dimens().get_value::<ValueDimenKey, Float>().cloned(), Some(12.))
}
