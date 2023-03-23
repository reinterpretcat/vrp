use super::*;
use crate::construction::heuristics::RouteContext;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{IdDimension, ValueDimension};
use crate::models::solution::Activity;

const VIOLATION_CODE: ViolationCode = 1;

fn create_single_with_order(id: &str, order: Option<f64>) -> Arc<Single> {
    let mut single = test_single();
    single.dimens.set_id(id);

    if let Some(order) = order {
        single.dimens.set_value("order", order);
    }

    Arc::new(single)
}

fn create_activity_for_job_with_order(id: &str, order: Option<f64>) -> Activity {
    Activity { job: Some(create_single_with_order(id, order)), ..test_activity() }
}

#[test]
fn can_get_violations() {
    let fleet = test_fleet();

    let route = RouteContext::new_with_state(
        Arc::new(create_route_with_activities(
            &fleet,
            "v1",
            vec![
                create_activity_for_job_with_order("job1", Some(2.)),
                create_activity_for_job_with_order("job2", None),
                create_activity_for_job_with_order("job3", Some(1.)),
            ],
        )),
        Arc::new(RouteState::default()),
    );

    let violations = get_violations(&[route], &get_order_fn());

    assert_eq!(violations, 1);
}

parameterized_test! {can_merge_order, (source, candidate, expected), {
    can_merge_order_impl(source, candidate, expected);
}}

can_merge_order! {
    case_01: (Some(1.), Some(1.), Ok(Some(1.))),
    case_02: (None, None, Ok(None)),
    case_03: (Some(1.), None, Err(VIOLATION_CODE)),
    case_04: (None, Some(1.), Err(VIOLATION_CODE)),
    case_05: (Some(1.), Some(2.), Err(VIOLATION_CODE)),
}

fn can_merge_order_impl(source: Option<f64>, candidate: Option<f64>, expected: Result<Option<f64>, i32>) {
    let constraint =
        create_tour_order_hard_feature("tour_order", VIOLATION_CODE, get_order_fn()).unwrap().constraint.unwrap();
    let source_job = Job::Single(create_single_with_order("source", source));
    let candidate_job = Job::Single(create_single_with_order("candidate", candidate));

    let result =
        constraint.merge(source_job, candidate_job).map(|merged| merged.dimens().get_value::<f64>("order").cloned());

    assert_eq!(result, expected);
}

fn get_order_fn() -> TourOrderFn {
    Either::Left(Arc::new(|single| {
        single.map_or(OrderResult::Ignored, |single| match single.dimens.get_value::<f64>("order") {
            Some(value) => OrderResult::Value(*value),
            _ => OrderResult::Default,
        })
    }))
}
