use crate::helpers::models::problem::{test_driver, test_vehicle_detail, FleetBuilder, VehicleBuilder};
use crate::models::common::TimeWindow;
use crate::models::problem::{Actor, VehicleDetail};
use crate::models::solution::Registry;
use std::cmp::Ordering::Less;
use std::sync::Arc;

parameterized_test! {can_provide_available_actors_from_registry, (count, expected), {
    can_provide_available_actors_from_registry_impl(count, expected);
}}

can_provide_available_actors_from_registry! {
    case1: (0, 3),
    case2: (1, 2),
    case3: (2, 1),
    case4: (3, 0),
}

fn can_provide_available_actors_from_registry_impl(count: usize, expected: usize) {
    let fleet = FleetBuilder::new()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::new().id("v1").details(vec![test_vehicle_detail()]).build(),
            VehicleBuilder::new()
                .id("v2")
                .details(vec![
                    test_vehicle_detail(),
                    VehicleDetail { start: Some(1), end: Some(0), time: Some(TimeWindow { start: 0.0, end: 50.0 }) },
                ])
                .build(),
        ])
        .build();
    let mut registry = Registry::new(&fleet);

    let actors: Vec<Arc<Actor>> = registry.available().take(count).collect();
    actors.iter().for_each(|a| registry.use_actor(a));
    assert_eq!(registry.available().count(), expected);
}

#[test]
fn can_provide_next_actors_from_registry() {
    let fleet = FleetBuilder::new()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::new().id("v1").details(vec![test_vehicle_detail()]).build(),
            VehicleBuilder::new()
                .id("v2")
                .details(vec![
                    test_vehicle_detail(),
                    VehicleDetail { start: Some(1), end: Some(0), time: Some(TimeWindow { start: 0.0, end: 50.0 }) },
                ])
                .build(),
            VehicleBuilder::new().id("v3").details(vec![test_vehicle_detail()]).build(),
        ])
        .build();
    let registry = Registry::new(&fleet);

    let mut actors: Vec<Arc<Actor>> = registry.next().collect();
    actors.sort_by(|a, b| a.detail.start.partial_cmp(&b.detail.start).unwrap_or(Less));
    assert_eq!(actors.len(), 2);
    assert_eq!(actors.first().unwrap().detail.start, Some(0));
    assert_eq!(actors.last().unwrap().detail.start, Some(1));
}
