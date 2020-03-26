use crate::models::common::{Cost, Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::Actor;
use crate::models::solution::Activity;

/// Provides the way to get cost information for specific activities done by specific actor.
pub trait ActivityCost {
    /// Returns cost to perform activity.
    fn cost(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost {
        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0.0 };
        let service = self.duration(actor, activity, arrival);

        waiting * (actor.driver.costs.per_waiting_time + actor.vehicle.costs.per_waiting_time)
            + service * (actor.driver.costs.per_service_time + actor.vehicle.costs.per_service_time)
    }

    /// Returns operation time spent to perform activity.
    fn duration(&self, _actor: &Actor, activity: &Activity, _arrival: Timestamp) -> Cost {
        activity.place.duration
    }
}

/// Default activity costs.
pub struct SimpleActivityCost {}

impl Default for SimpleActivityCost {
    fn default() -> Self {
        Self {}
    }
}

impl ActivityCost for SimpleActivityCost {}

/// Provides the way to get routing information for specific locations and actor.
pub trait TransportCost {
    /// Returns transport cost between two locations.
    fn cost(&self, actor: &Actor, from: Location, to: Location, departure: Timestamp) -> Cost {
        let distance = self.distance(actor.vehicle.profile, from, to, departure);
        let duration = self.duration(actor.vehicle.profile, from, to, departure);

        distance * (actor.driver.costs.per_distance + actor.vehicle.costs.per_distance)
            + duration * (actor.driver.costs.per_driving_time + actor.vehicle.costs.per_driving_time)
    }

    /// Returns transport time between two locations.
    fn duration(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Duration;

    /// Returns transport distance between two locations.
    fn distance(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Distance;
}

/// A transport cost implementation which uses custom distance and duration matrices as source of
/// transport cost information.
/// NOTE Not day time aware as it ignores departure timestamp.
pub struct MatrixTransportCost {
    durations: Vec<Vec<Duration>>,
    distances: Vec<Vec<Distance>>,
    size: usize,
}

pub struct MatrixCosts {
    /// A routing profile.
    pub profile: Profile,
    /// A timestamp for which routing info is applicable.
    pub timestamp: Timestamp,
    /// Travel durations.
    pub durations: Vec<Duration>,
    /// Travel distances.
    pub distances: Vec<Distance>,
}

impl MatrixTransportCost {
    /// Creates a new [`MatrixTransportCost`]
    pub fn new(durations: Vec<Vec<Duration>>, distances: Vec<Vec<Distance>>) -> Self {
        let size = (durations.first().unwrap().len() as f64).sqrt() as usize;

        assert_eq!(distances.len(), durations.len());
        assert!(distances.iter().all(|d| (d.len() as f64).sqrt() as usize == size));
        assert!(durations.iter().all(|d| (d.len() as f64).sqrt() as usize == size));

        Self { durations, distances, size }
    }
}

impl TransportCost for MatrixTransportCost {
    fn duration(&self, profile: Profile, from: Location, to: Location, _: Timestamp) -> Duration {
        *self.durations.get(profile as usize).unwrap().get(from * self.size + to).unwrap()
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, _: Timestamp) -> Distance {
        *self.distances.get(profile as usize).unwrap().get(from * self.size + to).unwrap()
    }
}
