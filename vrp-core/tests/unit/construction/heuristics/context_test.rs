use crate::construction::heuristics::{RouteState, StateKeyRegistry, UnassignmentInfo};
use crate::helpers::construction::heuristics::{create_schedule_keys, create_state_key, InsertionContextBuilder};
use crate::helpers::models::domain::GoalContextBuilder;
use crate::helpers::models::problem::{test_fleet, SingleBuilder};
use crate::helpers::models::solution::*;

#[test]
fn can_put_and_get_activity_states_with_different_keys() {
    let mut keys = StateKeyRegistry::default();
    let (key1, key2, key3, key4) = (keys.next_key(), keys.next_key(), keys.next_key(), keys.next_key());
    let mut route_state = RouteState::default();

    route_state.put_activity_states(key1, vec!["key1".to_string()]);
    route_state.put_activity_states(key2, vec!["key2".to_string()]);
    route_state.put_activity_states(key3, vec!["key3".to_string()]);
    let result3 = route_state.get_activity_state::<String>(key3, 0);
    let result1 = route_state.get_activity_state::<String>(key1, 0);
    let result2 = route_state.get_activity_state::<String>(key2, 0);
    let result4 = route_state.get_activity_state::<String>(key4, 0);

    assert_eq!(result1.unwrap(), "key1");
    assert_eq!(result2.unwrap(), "key2");
    assert_eq!(result3.unwrap(), "key3");
    assert!(result4.is_none());
}

#[test]
fn can_put_and_get_route_state() {
    let state_key = create_state_key();
    let mut route_state = RouteState::default();

    route_state.put_route_state(state_key, "my_value".to_string());
    let result = route_state.get_route_state::<String>(state_key);

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_empty_route_state() {
    let mut keys = StateKeyRegistry::default();
    let (key1, key2) = (keys.next_key(), keys.next_key());
    let mut route_state = RouteState::default();

    route_state.put_route_state(key1, "my_value".to_string());
    let result = route_state.get_route_state::<String>(key2);

    assert!(result.is_none());
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
    let insertion_ctx = InsertionContextBuilder::default()
        .with_goal(GoalContextBuilder::with_transport_feature(create_schedule_keys()).build())
        .with_routes(vec![RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .add_activity(ActivityBuilder::default().build())
                    .with_vehicle(&test_fleet(), "v1")
                    .build(),
            )
            .build()])
        .with_unassigned(vec![(SingleBuilder::default().build_as_job_ref(), UnassignmentInfo::Unknown)])
        .build();

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
