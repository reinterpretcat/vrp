use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::problem::*;
use crate::models::problem::{Actor, ActorDetail, Fleet};
use crate::models::solution::Registry;
use std::sync::Arc;

pub fn test_actor() -> Arc<Actor> {
    Arc::new(Actor {
        vehicle: Arc::new(test_vehicle(DEFAULT_PROFILE)),
        driver: Arc::new(test_driver()),
        detail: ActorDetail {
            start: Some(DEFAULT_ACTOR_LOCATION),
            end: Some(DEFAULT_ACTOR_LOCATION),
            time: DEFAULT_ACTOR_TIME_WINDOW,
        },
    })
}

pub fn create_test_registry() -> Registry {
    Registry::new(&Fleet::new(
        vec![test_driver_with_costs(empty_costs())],
        vec![VehicleBuilder::new().id("v1").build()],
    ))
}
