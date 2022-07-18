use crate::models::common::{Distance, Duration};
use crate::models::problem::{Actor, TravelLimits};
use std::ops::Deref;
use std::sync::Arc;

/// No travel limits for any actor.
#[derive(Default)]
pub struct NoTravelLimits {}

impl TravelLimits for NoTravelLimits {
    fn get_global_duration(&self, _: &Actor) -> Option<Duration> {
        None
    }

    fn get_global_distance(&self, _: &Actor) -> Option<Distance> {
        None
    }
}

/// A simple travel limits implementation.
pub struct SimpleTravelLimits {
    distance: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
    duration: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
}

impl SimpleTravelLimits {
    /// Creates a new instance of `SimpleTravelLimits`.
    pub fn new(
        distance: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
        duration: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
    ) -> Self {
        Self { distance, duration }
    }
}

impl TravelLimits for SimpleTravelLimits {
    fn get_global_duration(&self, actor: &Actor) -> Option<Duration> {
        self.duration.deref()(actor)
    }

    fn get_global_distance(&self, actor: &Actor) -> Option<Distance> {
        self.distance.deref()(actor)
    }
}
