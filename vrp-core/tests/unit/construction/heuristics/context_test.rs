use crate::construction::heuristics::{RegistryContext, RouteState, UnassignmentInfo};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::{TestGoalContextBuilder, test_random};
use crate::helpers::models::problem::{FleetBuilder, TestSingleBuilder, TestVehicleBuilder, test_driver, test_fleet};
use crate::helpers::models::solution::*;
use crate::models::solution::Registry;

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
        .with_routes(vec![
            RouteContextBuilder::default()
                .with_route(
                    RouteBuilder::default()
                        .add_activity(ActivityBuilder::default().build())
                        .with_vehicle(&test_fleet(), "v1")
                        .build(),
                )
                .build(),
        ])
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

#[test]
fn can_only_use_routes_retained_by_deep_slice() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            TestVehicleBuilder::default().id("v1").build(),
            TestVehicleBuilder::default().id("v2").build(),
        ])
        .build();
    let retained = fleet.actors[0].clone();
    let excluded = fleet.actors[1].clone();
    let registry = Registry::new(&fleet, test_random());
    let mut registry = RegistryContext::new(&TestGoalContextBuilder::default().build(), registry)
        .deep_slice(|actor| std::ptr::eq(actor, retained.as_ref()));

    assert_eq!(registry.next_route().count(), 1);
    assert!(registry.get_route(&excluded).is_none());
    assert!(registry.get_route(&retained).is_some());
}
