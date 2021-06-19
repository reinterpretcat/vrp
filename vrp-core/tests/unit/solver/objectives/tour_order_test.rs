use super::*;
use crate::construction::heuristics::RouteContext;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::ValueDimension;
use crate::models::solution::Activity;

fn create_activity_for_job_with_order(order: Option<i32>) -> Activity {
    let mut single = test_single();

    if let Some(order) = order {
        single.dimens.insert("order".to_string(), Arc::new(order as f64));
    }

    Activity { job: Some(Arc::new(single)), ..test_activity() }
}

#[test]
fn can_get_violations() {
    let fleet = test_fleet();

    let route = RouteContext::new_with_state(
        Arc::new(create_route_with_activities(
            &fleet,
            "v1",
            vec![
                create_activity_for_job_with_order(Some(2)),
                create_activity_for_job_with_order(None),
                create_activity_for_job_with_order(Some(1)),
            ],
        )),
        Arc::new(RouteState::default()),
    );

    let violations = get_violations(&[route], &|single| single.dimens.get_value::<f64>("order").cloned());

    assert_eq!(violations, 1);
}
