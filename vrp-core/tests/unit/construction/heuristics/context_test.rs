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
fn can_reuse_exclusive_route_state() {
    let mut route_state = RouteState::default();
    route_state.set_tour_state::<i8, _>(vec![1]);

    route_state.get_or_init_exclusive_tour_state::<i8, Vec<i32>>(|| panic!("state should be reused")).push(2);

    assert_eq!(route_state.get_tour_state::<i8, Vec<i32>>().unwrap(), &[1, 2]);
}

#[test]
fn can_replace_shared_route_state_without_changing_original() {
    let mut route_state = RouteState::default();
    route_state.set_tour_state::<i8, _>(vec![1]);
    let original = route_state.clone();

    route_state.get_or_init_exclusive_tour_state::<i8, Vec<i32>>(|| vec![2]).push(3);

    assert_eq!(original.get_tour_state::<i8, Vec<i32>>().unwrap(), &[1]);
    assert_eq!(route_state.get_tour_state::<i8, Vec<i32>>().unwrap(), &[2, 3]);
}

#[test]
fn can_update_exclusive_route_state() {
    let mut route_state = RouteState::default();
    route_state.set_tour_state::<i8, _>(1);

    route_state.update_tour_state::<i8, _>(2);

    assert_eq!(route_state.get_tour_state::<i8, i32>(), Some(&2));
}

#[test]
fn can_update_shared_route_state_without_changing_original() {
    let mut route_state = RouteState::default();
    route_state.set_tour_state::<i8, _>(1);
    let original = route_state.clone();

    route_state.update_tour_state::<i8, _>(2);

    assert_eq!(original.get_tour_state::<i8, i32>(), Some(&1));
    assert_eq!(route_state.get_tour_state::<i8, i32>(), Some(&2));
}

#[test]
fn can_set_and_get_empty_route_state() {
    let mut route_state = RouteState::default();

    route_state.set_tour_state::<i8, _>("my_value".to_string());
    let result = route_state.get_tour_state::<i16, String>();

    assert!(result.is_none());
}

#[test]
fn can_mutate_deep_copied_route_state_independently() {
    let mut original = RouteContextBuilder::default().build();
    original.state_mut().set_tour_state::<i8, _>("original".to_string());

    let mut copy = original.deep_copy();
    copy.state_mut().set_tour_state::<i8, _>("copy".to_string());

    assert_eq!(original.state().get_tour_state::<i8, String>().unwrap(), "original");
    assert_eq!(copy.state().get_tour_state::<i8, String>().unwrap(), "copy");
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
    let mut registry = RegistryContext::new(&TestGoalContextBuilder::default().build(), registry);
    let excluded_route = registry.get_route(&excluded).unwrap();
    let mut registry = registry.deep_slice(|actor| std::ptr::eq(actor, retained.as_ref()));

    assert_eq!(registry.next_route().count(), 1);
    assert!(registry.get_route(&excluded).is_none());
    assert!(!registry.free_route(excluded_route));
    assert!(registry.get_route(&retained).is_some());
}

#[test]
fn can_reuse_used_route_retained_by_deep_slice() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            TestVehicleBuilder::default().id("v1").build(),
            TestVehicleBuilder::default().id("v2").build(),
        ])
        .build();
    let retained = fleet.actors[0].clone();
    let registry = Registry::new(&fleet, test_random());
    let mut registry = RegistryContext::new(&TestGoalContextBuilder::default().build(), registry);
    let route = registry.get_route(&retained).unwrap();
    let mut registry = registry.deep_slice(|actor| std::ptr::eq(actor, retained.as_ref()));

    assert_eq!(registry.next_route().count(), 0);
    assert!(registry.free_route(route));
    assert_eq!(registry.next_route().count(), 1);
}

#[test]
fn can_copy_registry_with_all_actors_available() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            TestVehicleBuilder::default().id("v1").build(),
            TestVehicleBuilder::default().id("v2").build(),
        ])
        .build();
    let actors = fleet.actors.clone();
    let registry = Registry::new(&fleet, test_random());
    let mut registry = RegistryContext::new(&TestGoalContextBuilder::default().build(), registry);
    assert!(registry.get_route(&actors[0]).is_some());

    let registry = registry.deep_copy_with_all_available();

    assert_eq!(registry.resources().available().count(), actors.len());
}

#[test]
fn can_copy_sliced_registry_with_only_retained_actors_available() {
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
    let registry = RegistryContext::new(&TestGoalContextBuilder::default().build(), registry)
        .deep_slice(|actor| std::ptr::eq(actor, retained.as_ref()))
        .deep_copy_with_all_available();

    assert_eq!(registry.resources().available().count(), 1);
    assert!(registry.resources().available().any(|actor| actor == retained));
    assert!(registry.resources().all().all(|actor| actor != excluded));
}
