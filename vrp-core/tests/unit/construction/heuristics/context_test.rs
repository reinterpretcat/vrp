use crate::construction::heuristics::{RouteState, UnassignmentInfo};
use crate::helpers::construction::features::create_goal_ctx_with_transport;
use crate::helpers::construction::heuristics::create_insertion_context;
use crate::helpers::models::domain::test_random;
use crate::helpers::models::problem::{test_fleet, SingleBuilder};
use crate::helpers::models::solution::*;
use crate::models::solution::Registry;

#[test]
fn can_put_and_get_activity_state() {
    let mut route_state = RouteState::default();
    let activity = test_activity();

    route_state.put_activity_state(1, &activity, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(1, &activity);

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_empty_activity_state() {
    let mut route_state = RouteState::default();
    let activity1 = test_activity();
    let activity2 = test_activity();

    route_state.put_activity_state(1, &activity1, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(1, &activity2);

    assert!(result.is_none());
}

#[test]
fn can_put_and_get_activity_state_with_different_keys() {
    let mut route_state = RouteState::default();
    let activity = test_activity();

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
fn can_put_and_get_route_state() {
    let mut route_state = RouteState::default();

    route_state.put_route_state(1, "my_value".to_string());
    let result = route_state.get_route_state::<String>(1);

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_empty_route_state() {
    let mut route_state = RouteState::default();

    route_state.put_route_state(1, "my_value".to_string());
    let result = route_state.get_route_state::<String>(2);

    assert!(result.is_none());
}

#[test]
fn can_remove_activity_states() {
    let mut route_state = RouteState::default();
    let activity = test_activity();

    route_state.put_activity_state(1, &activity, "key1".to_string());
    route_state.put_activity_state(2, &activity, "key2".to_string());
    route_state.remove_activity_states(&activity);
    let result1 = route_state.get_activity_state::<String>(1, &activity);
    let result2 = route_state.get_activity_state::<String>(2, &activity);

    assert!(result1.is_none());
    assert!(result2.is_none());
}

#[test]
fn can_use_stale_flag() {
    let mut route_ctx = RouteContextBuilder::default().build();

    assert!(route_ctx.is_stale());
    route_ctx.mark_stale(false);
    assert!(!route_ctx.is_stale());

    let mut route_ctx = RouteContextBuilder::default().build();
    route_ctx.mark_stale(false);
    let _ = route_ctx.as_mut();
    assert!(route_ctx.is_stale());
}

#[test]
fn can_use_debug_fmt_for_insertion_ctx() {
    let fleet = test_fleet();
    let mut insertion_ctx = create_insertion_context(
        Registry::new(&fleet, test_random()),
        create_goal_ctx_with_transport(),
        vec![RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activity(test_activity()).build())
            .build()],
    );
    insertion_ctx.solution.unassigned.insert(SingleBuilder::default().build_as_job_ref(), UnassignmentInfo::Unknown);

    let result = format!("{insertion_ctx:#?}");

    println!("{result}");
    assert!(!result.contains("::"));
    assert!(result.contains("tour"));
    assert!(result.contains("vehicle: \"v1\""));
    assert!(result.contains("departure"));
    assert!(result.contains("arrival"));

    assert!(result.contains("unassigned"));
    assert!(result.contains("id: \"single\""));
}
