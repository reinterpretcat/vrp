use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::problem::{test_driver, test_vehicle, DEFAULT_ACTOR_LOCATION, DEFAULT_ACTOR_TIME_WINDOW};
use crate::models::solution::{Actor, Detail};
use std::sync::Arc;

pub fn test_actor() -> Arc<Actor> {
    Arc::new(Actor {
        vehicle: Arc::new(test_vehicle(DEFAULT_PROFILE)),
        driver: Arc::new(test_driver()),
        detail: Detail {
            start: Some(DEFAULT_ACTOR_LOCATION),
            end: Some(DEFAULT_ACTOR_LOCATION),
            time: DEFAULT_ACTOR_TIME_WINDOW,
        },
    })
}
