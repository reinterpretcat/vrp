use crate::helpers::models::domain::test_random;
use crate::helpers::models::problem::*;
use crate::models::common::TimeInterval;
use crate::models::problem::{Actor, ActorDetail, VehiclePlace};
use crate::models::solution::Registry;
use std::sync::Arc;

pub fn test_actor() -> Arc<Actor> {
    test_actor_with_profile(0)
}

pub fn test_actor_with_profile(profile_idx: usize) -> Arc<Actor> {
    Arc::new(Actor {
        vehicle: Arc::new(test_vehicle(profile_idx)),
        driver: Arc::new(test_driver()),
        detail: ActorDetail {
            start: Some(VehiclePlace {
                location: DEFAULT_ACTOR_LOCATION,
                time: TimeInterval { earliest: Some(DEFAULT_ACTOR_TIME_WINDOW.start), latest: None },
            }),
            end: Some(VehiclePlace {
                location: DEFAULT_ACTOR_LOCATION,
                time: TimeInterval { earliest: None, latest: Some(DEFAULT_ACTOR_TIME_WINDOW.end) },
            }),
            time: DEFAULT_ACTOR_TIME_WINDOW,
        },
    })
}

pub fn create_test_registry() -> Registry {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver_with_costs(empty_costs()))
        .add_vehicle(TestVehicleBuilder::default().id("v1").build())
        .build();
    Registry::new(&fleet, test_random())
}
