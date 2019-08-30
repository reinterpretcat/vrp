use crate::construction::states::route::RouteState;
use crate::helpers::models::solution::test_activity;
use crate::models::solution::Activity;
use std::sync::Arc;

fn new_activity_ref() -> Arc<Activity> {
    Arc::new(test_activity())
}

#[test]
fn can_put_and_get_activity_state() {
    let mut route_state = RouteState::new((1, 1));
    let activity = new_activity_ref();

    route_state.put_activity_state(1, &activity, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(1, &activity);

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_activity_state_with_different_keys() {
    let mut route_state = RouteState::new((1, 1));
    let activity = new_activity_ref();

    route_state.put_activity_state(1, &activity, "key1".to_string());
    route_state.put_activity_state(2, &activity, "key2".to_string());
    route_state.put_activity_state(3, &activity, "key3".to_string());
    let result3 = route_state.get_activity_state::<String>(3, &activity);
    let result1 = route_state.get_activity_state::<String>(1, &activity);
    let result2 = route_state.get_activity_state::<String>(2, &activity);
    let result4 = route_state.get_activity_state::<String>(4, &activity);

    assert_eq!(result1.unwrap(), "key1");
    assert_eq!(result2.unwrap(), "key2");
    assert_eq!(result3.unwrap(), "key3");
    assert!(result4.is_none());
}

#[test]
fn can_put_and_get_empty_state() {
    let mut route_state = RouteState::new((1, 1));
    let activity1 = new_activity_ref();
    let activity2 = new_activity_ref();

    route_state.put_activity_state(1, &activity1, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(1, &activity2);

    assert!(result.is_none());
}
