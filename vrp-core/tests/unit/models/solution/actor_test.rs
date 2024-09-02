use crate::helpers::models::domain::test_random;
use crate::helpers::models::problem::{test_driver, test_vehicle_detail, FleetBuilder, TestVehicleBuilder};
use crate::models::common::TimeInterval;
use crate::models::problem::{Actor, VehicleDetail, VehiclePlace};
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
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            TestVehicleBuilder::default().id("v1").details(vec![test_vehicle_detail()]).build(),
            TestVehicleBuilder::default().id("v2").details(create_two_test_vehicle_details()).build(),
        ])
        .build();
    let mut registry = Registry::new(&fleet, test_random());

    let actors: Vec<Arc<Actor>> = registry.available().take(count).collect();
    actors.iter().for_each(|a| {
        registry.use_actor(a);
    });
    assert_eq!(registry.available().count(), expected);
}

#[test]
fn can_provide_next_actors_from_registry() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            TestVehicleBuilder::default().id("v1").details(vec![test_vehicle_detail()]).build(),
            TestVehicleBuilder::default().id("v2").details(create_two_test_vehicle_details()).build(),
            TestVehicleBuilder::default().id("v3").details(vec![test_vehicle_detail()]).build(),
        ])
        .build();
    let registry = Registry::new(&fleet, test_random());

    let mut actors: Vec<Arc<Actor>> = registry.next().collect();
    actors.sort_by(|a, b| {
        let a = a.detail.start.as_ref().map(|s| s.location);
        let b = b.detail.start.as_ref().map(|s| s.location);
        a.partial_cmp(&b).unwrap_or(Less)
    });
    assert_eq!(actors.len(), 2);
    assert_eq!(actors.first().unwrap().detail.start.as_ref().map(|s| s.location), Some(0));
    assert_eq!(actors.last().unwrap().detail.start.as_ref().map(|s| s.location), Some(1));
}

fn create_two_test_vehicle_details() -> Vec<VehicleDetail> {
    vec![
        test_vehicle_detail(),
        VehicleDetail {
            start: Some(VehiclePlace { location: 1, time: TimeInterval { earliest: Some(0), latest: None } }),
            end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(50) } }),
        },
    ]
}
