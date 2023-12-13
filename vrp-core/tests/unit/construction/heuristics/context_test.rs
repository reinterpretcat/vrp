use crate::construction::heuristics::{RouteState, StateKey, UnassignmentInfo};
use crate::helpers::construction::features::create_goal_ctx_with_transport;
use crate::helpers::construction::heuristics::create_insertion_context;
use crate::helpers::models::domain::test_random;
use crate::helpers::models::problem::{test_fleet, SingleBuilder};
use crate::helpers::models::solution::*;
use crate::models::solution::Registry;

#[test]
fn can_put_and_get_activity_state() {
    let mut route_state = RouteState::default();

    route_state.put_activity_state(StateKey(1), 0, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(StateKey(1), 0);

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_empty_activity_state() {
    let mut route_state = RouteState::default();

    route_state.put_activity_state(StateKey(1), 0, "my_value".to_string());
    let result = route_state.get_activity_state::<String>(StateKey(1), 1);

    assert!(result.is_none());
}

#[test]
fn can_put_and_get_activity_state_with_different_keys() {
    let mut route_state = RouteState::default();

    route_state.put_activity_state(StateKey(1), 0, "key1".to_string());
    route_state.put_activity_state(StateKey(2), 0, "key2".to_string());
    route_state.put_activity_state(StateKey(3), 0, "key3".to_string());
    let result3 = route_state.get_activity_state::<String>(StateKey(3), 0);
    let result1 = route_state.get_activity_state::<String>(StateKey(1), 0);
    let result2 = route_state.get_activity_state::<String>(StateKey(2), 0);
    let result4 = route_state.get_activity_state::<String>(StateKey(4), 0);

    assert_eq!(result1.unwrap(), "key1");
    assert_eq!(result2.unwrap(), "key2");
    assert_eq!(result3.unwrap(), "key3");
    assert!(result4.is_none());
}

#[test]
fn can_put_and_get_route_state() {
    let mut route_state = RouteState::default();

    route_state.put_route_state(StateKey(1), "my_value".to_string());
    let result = route_state.get_route_state::<String>(StateKey(1));

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_put_and_get_empty_route_state() {
    let mut route_state = RouteState::default();

    route_state.put_route_state(StateKey(1), "my_value".to_string());
    let result = route_state.get_route_state::<String>(StateKey(2));

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
    let fleet = test_fleet();
    let mut insertion_ctx = create_insertion_context(
        Registry::new(&fleet, test_random()),
        create_goal_ctx_with_transport(),
        vec![RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(ActivityBuilder::default().build())
                    .build(),
            )
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
