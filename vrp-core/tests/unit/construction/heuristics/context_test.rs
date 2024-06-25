use crate::construction::heuristics::{RouteState, UnassignmentInfo};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::problem::{test_fleet, TestSingleBuilder};
use crate::helpers::models::solution::*;

#[test]
fn can_set_and_get_activity_states_with_different_type_keys() {
    let mut route_state = RouteState::default();

    route_state.set_activity_states::<i8, _>(vec!["key1".to_string()]);
    route_state.set_activity_states::<i16, _>(vec!["key2".to_string()]);
    route_state.set_activity_states::<i32, _>(vec!["key3".to_string()]);
    let result3 = route_state.get_activity_state::<i32, String>(0);
    let result1 = route_state.get_activity_state::<i8, String>(0);
    let result2 = route_state.get_activity_state::<i16, String>(0);
    let result4 = route_state.get_activity_state::<i64, String>(0);

    assert_eq!(result1.unwrap(), "key1");
    assert_eq!(result2.unwrap(), "key2");
    assert_eq!(result3.unwrap(), "key3");
    assert!(result4.is_none());
}

#[test]
fn can_set_and_get_route_state() {
    let mut route_state = RouteState::default();

    route_state.set_tour_state::<(), _>("my_value".to_string());
    let result = route_state.get_tour_state::<(), String>();

    assert_eq!(result.unwrap(), "my_value");
}

#[test]
fn can_set_and_get_empty_route_state() {
    let mut route_state = RouteState::default();

    route_state.set_tour_state::<i8, _>("my_value".to_string());
    let result = route_state.get_tour_state::<i16, String>();

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
    let insertion_ctx = TestInsertionContextBuilder::default()
        .with_goal(TestGoalContextBuilder::with_transport_feature().build())
        .with_routes(vec![RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .add_activity(ActivityBuilder::default().build())
                    .with_vehicle(&test_fleet(), "v1")
                    .build(),
            )
            .build()])
        .with_unassigned(vec![(TestSingleBuilder::default().build_as_job_ref(), UnassignmentInfo::Unknown)])
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
