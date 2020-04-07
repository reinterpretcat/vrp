#[cfg(test)]
#[path = "../../../tests/unit/models/problem/costs_test.rs"]
mod costs_test;

use crate::models::common::{Cost, Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::Actor;
use crate::models::solution::Activity;
use crate::utils::CollectGroupBy;
use hashbrown::HashMap;

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

/// Contains matrix routing data for specific profile and, optionally, time.
pub struct MatrixData {
    /// A routing profile.
    pub profile: Profile,
    /// A timestamp for which routing info is applicable.
    pub timestamp: Option<Timestamp>,
    /// Travel durations.
    pub durations: Vec<Duration>,
    /// Travel distances.
    pub distances: Vec<Distance>,
}

impl MatrixTransportCost {
    /// Creates a new [`MatrixTransportCost`]
    pub fn new(costs: Vec<MatrixData>) -> Self {
        let mut costs = costs;
        costs.sort_by(|a, b| a.profile.cmp(&b.profile));

        if costs.iter().any(|costs| costs.timestamp.is_some()) {
            unimplemented!("Time aware routing is not yet implemented")
        }

        if (0..).zip(costs.iter().map(|c| c.profile)).any(|(a, b)| a != b) {
            unimplemented!("Duplicate profiles can be passed only for time aware routing")
        }

        let (durations, distances) = costs.into_iter().fold((vec![], vec![]), |mut acc, data| {
            acc.0.push(data.durations);
            acc.1.push(data.distances);

            acc
        });

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

impl MatrixData {
    /// Creates `MatrixData` without timestamp.
    pub fn new(profile: Profile, durations: Vec<Duration>, distances: Vec<Distance>) -> Self {
        Self { profile, timestamp: None, durations, distances }
    }
}

/// A time aware matrix costs.
struct TimeAwareMatrixTransportCost {
    costs: HashMap<Profile, (Vec<u64>, Vec<MatrixData>)>,
    size: usize,
}

impl TimeAwareMatrixTransportCost {
    /// Creates a new [`TimeAwareMatrixTransportCost`]
    fn new(costs: Vec<MatrixData>, size: usize) -> Result<Self, String> {
        if costs.iter().any(|matrix| matrix.timestamp.is_none()) {
            return Err("Cannot use matrix without timestamp".to_string());
        }

        let costs = costs.into_iter().collect_group_by_key(|matrix| matrix.profile);

        if costs.iter().any(|(_, matrices)| matrices.len() == 1) {
            return Err("Should not use time aware matrix routing with single matrix".to_string());
        }

        let costs = costs
            .into_iter()
            .map(|(profile, mut matrices)| {
                matrices.sort_by(|a, b| (a.timestamp.unwrap() as u64).cmp(&(b.timestamp.unwrap() as u64)));
                let timestamps = matrices.iter().map(|matrix| matrix.timestamp.unwrap() as u64).collect();

                (profile, (timestamps, matrices))
            })
            .collect();

        Ok(Self { costs, size })
    }
}

impl TransportCost for TimeAwareMatrixTransportCost {
    fn duration(&self, profile: Profile, from: Location, to: Location, timestamp: Timestamp) -> Duration {
        let (timestamps, matrices) = self.costs.get(&profile).unwrap();
        let data_idx = from * self.size + to;

        match timestamps.binary_search(&(timestamp as u64)) {
            Ok(matrix_idx) => *matrices.get(matrix_idx).unwrap().durations.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == 0 => *matrices.first().unwrap().durations.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == matrices.len() => {
                *matrices.last().unwrap().durations.get(data_idx).unwrap()
            }
            Err(matrix_idx) => {
                let left_matrix = matrices.get(matrix_idx - 1).unwrap();
                let right_matrix = matrices.get(matrix_idx).unwrap();

                let left_value = *matrices.get(matrix_idx - 1).unwrap().durations.get(data_idx).unwrap();
                let right_value = *matrices.get(matrix_idx).unwrap().durations.get(data_idx).unwrap();

                // perform linear interpolation
                let ratio = (timestamp - left_matrix.timestamp.unwrap())
                    / (right_matrix.timestamp.unwrap() - left_matrix.timestamp.unwrap());

                left_value + ratio * (right_value - left_value)
            }
        }
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, timestamp: Timestamp) -> Distance {
        let (timestamps, matrices) = self.costs.get(&profile).unwrap();
        let data_idx = from * self.size + to;

        match timestamps.binary_search(&(timestamp as u64)) {
            Ok(matrix_idx) => *matrices.get(matrix_idx).unwrap().distances.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == 0 => *matrices.first().unwrap().distances.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == matrices.len() => {
                *matrices.last().unwrap().distances.get(data_idx).unwrap()
            }
            Err(matrix_idx) => *matrices.get(matrix_idx).unwrap().distances.get(data_idx).unwrap(),
        }
    }
}
