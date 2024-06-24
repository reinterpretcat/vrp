use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::solution::Activity;

const VIOLATION_CODE: ViolationCode = 1;

struct OrderDimenKey;

fn create_single_with_order(id: &str, order: Option<f64>) -> Arc<Single> {
    let mut single = SingleBuilder::default().id(id).build();

    if let Some(order) = order {
        single.dimens.set_value::<OrderDimenKey, _>(order);
    }

    Arc::new(single)
}

fn create_activity_for_job_with_order(id: &str, order: Option<f64>) -> Activity {
    Activity { job: Some(create_single_with_order(id, order)), ..ActivityBuilder::default().build() }
}

#[test]
fn can_get_violations() {
    let fleet = test_fleet();
    let route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activity(create_activity_for_job_with_order("job1", Some(2.)))
                .add_activity(create_activity_for_job_with_order("job2", None))
                .add_activity(create_activity_for_job_with_order("job3", Some(1.)))
                .build(),
        )
        .build();

    let violations = get_violations(&[route_ctx], &get_order_fn());

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

    let result = constraint
        .merge(source_job, candidate_job)
        .map(|merged| merged.dimens().get_value::<OrderDimenKey, f64>().cloned());

    assert_eq!(result, expected);
}

fn get_order_fn() -> TourOrderFn {
    Either::Left(Arc::new(|single| match single.dimens.get_value::<OrderDimenKey, f64>() {
        Some(value) => OrderResult::Value(*value),
        _ => OrderResult::Default,
    }))
}
