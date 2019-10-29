use crate::models::common::{Cost, Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{Driver, Vehicle};
use crate::models::solution::Activity;

// TODO add default implementation
/// Provides the way to get cost information for specific activities.
pub trait ActivityCost {
    /// Returns cost to perform activity.
    fn cost(&self, vehicle: &Vehicle, driver: &Driver, activity: &Activity, arrival: Timestamp) -> Cost {
        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0.0 };
        let service = self.duration(vehicle, driver, activity, arrival);

        waiting * (driver.costs.per_waiting_time + vehicle.costs.per_waiting_time)
            + service * (driver.costs.per_service_time + vehicle.costs.per_service_time)
    }

    /// Returns operation time spent to perform activity.
    fn duration(&self, _vehicle: &Vehicle, _driver: &Driver, activity: &Activity, _arrival: Timestamp) -> Cost {
        activity.place.duration
    }
}

/// Default activity costs.
pub struct SimpleActivityCost {}

impl SimpleActivityCost {
    pub fn new() -> Self {
        Self {}
    }
}

impl ActivityCost for SimpleActivityCost {}

/// Provides the way to get routing information for specific locations.
pub trait TransportCost {
    /// Returns transport cost between two locations.
    fn cost(&self, vehicle: &Vehicle, driver: &Driver, from: Location, to: Location, departure: Timestamp) -> Cost {
        let distance = self.distance(vehicle.profile, from, to, departure);
        let duration = self.duration(vehicle.profile, from, to, departure);

        distance * (driver.costs.per_distance + vehicle.costs.per_distance)
            + duration * (driver.costs.per_driving_time + vehicle.costs.per_driving_time)
    }

    /// Returns transport time between two locations.
    fn duration(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Duration;

    /// Returns transport distance between two locations.
    fn distance(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Distance;
}

/// Uses custom distance and duration matrices as source of transport cost information.
/// Not time aware as it ignores departure timestamp.
pub struct MatrixTransportCost {
    durations: Vec<Vec<Duration>>,
    distances: Vec<Vec<Distance>>,
    size: usize,
}

impl MatrixTransportCost {
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
        self.durations.get(profile as usize).unwrap().get(from * self.size + to).unwrap().clone()
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, _: Timestamp) -> Distance {
        self.distances.get(profile as usize).unwrap().get(from * self.size + to).unwrap().clone()
    }
}
