#[cfg(test)]
#[path = "../../../tests/unit/models/problem/costs_test.rs"]
mod costs_test;

use crate::models::common::*;
use crate::models::solution::{Activity, Route};
use rosomaxa::prelude::GenericError;
use rosomaxa::utils::CollectGroupBy;
use std::collections::HashMap;
use std::sync::Arc;

/// Specifies travel time type.
#[derive(Copy, Clone)]
pub enum TravelTime {
    /// Arrival time type.
    Arrival(Timestamp),
    /// Departure time type
    Departure(Timestamp),
}

/// Provides the way to get cost information for specific activities done by specific actor.
pub trait ActivityCost {
    /// Returns cost to perform activity.
    fn cost(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Cost {
        let actor = route.actor.as_ref();

        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0. };
        let service = activity.place.duration;

        waiting * (actor.driver.costs.per_waiting_time + actor.vehicle.costs.per_waiting_time)
            + service * (actor.driver.costs.per_service_time + actor.vehicle.costs.per_service_time)
    }

    /// Estimates departure time for activity and actor at given arrival time.
    fn estimate_departure(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp;

    /// Estimates arrival time for activity and actor at given departure time.
    fn estimate_arrival(&self, route: &Route, activity: &Activity, departure: Timestamp) -> Timestamp;
}

/// An actor independent activity costs.
#[derive(Default)]
pub struct SimpleActivityCost {}

impl ActivityCost for SimpleActivityCost {
    fn estimate_departure(&self, _: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        arrival.max(activity.place.time.start) + activity.place.duration
    }

    fn estimate_arrival(&self, _: &Route, activity: &Activity, departure: Timestamp) -> Timestamp {
        activity.place.time.end.min(departure - activity.place.duration)
    }
}

/// Provides the way to get routing information for specific locations and actor.
pub trait TransportCost {
    /// Returns time-dependent transport cost between two locations for given actor.
    fn cost(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Cost {
        let actor = route.actor.as_ref();

        let distance = self.distance(route, from, to, travel_time);
        let duration = self.duration(route, from, to, travel_time);

        distance * (actor.driver.costs.per_distance + actor.vehicle.costs.per_distance)
            + duration * (actor.driver.costs.per_driving_time + actor.vehicle.costs.per_driving_time)
    }

    /// Returns time-independent travel duration between locations specific for given profile.
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration;

    /// Returns time-independent travel distance between locations specific for given profile.
    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance;

    /// Returns time-dependent travel duration between locations specific for given actor.
    fn duration(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Duration;

    /// Returns time-dependent travel distance between locations specific for given actor.
    fn distance(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Distance;
}

/// Contains matrix routing data for specific profile and, optionally, time.
pub struct MatrixData {
    /// A routing profile index.
    pub index: usize,
    /// A timestamp for which routing info is applicable.
    pub timestamp: Option<Timestamp>,
    /// Travel durations.
    pub durations: Vec<Duration>,
    /// Travel distances.
    pub distances: Vec<Distance>,
}

impl MatrixData {
    /// Creates `MatrixData` instance.
    pub fn new(index: usize, timestamp: Option<Timestamp>, durations: Vec<Duration>, distances: Vec<Distance>) -> Self {
        Self { index, timestamp, durations, distances }
    }
}

/// A fallback for transport costs if from->to entry is not defined.
pub trait TransportFallback: Send + Sync {
    /// Returns fallback duration.
    fn duration(&self, profile: &Profile, from: Location, to: Location) -> Duration;

    /// Returns fallback distance.
    fn distance(&self, profile: &Profile, from: Location, to: Location) -> Distance;
}

/// A trivial implementation of no fallback for transport cost.
struct NoFallback;

impl TransportFallback for NoFallback {
    fn duration(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        panic!("cannot get duration for {from}->{to} for {profile:?}")
    }

    fn distance(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        panic!("cannot get distance for {from}->{to} for {profile:?}")
    }
}

/// Creates time agnostic or time aware routing costs based on matrix data passed.
/// Panics at runtime if given route path is not present in matrix data.
pub fn create_matrix_transport_cost(
    costs: Vec<MatrixData>,
) -> Result<Arc<dyn TransportCost + Send + Sync>, GenericError> {
    create_matrix_transport_cost_with_fallback(costs, NoFallback)
}

/// Creates time agnostic or time aware routing costs based on matrix data passed using
/// a fallback function for unknown route.
pub fn create_matrix_transport_cost_with_fallback<T: TransportFallback + 'static>(
    costs: Vec<MatrixData>,
    fallback: T,
) -> Result<Arc<dyn TransportCost + Send + Sync>, GenericError> {
    if costs.is_empty() {
        return Err("no matrix data found".into());
    }

    let size = (costs.first().unwrap().durations.len() as f64).sqrt().round() as usize;

    if costs.iter().any(|matrix| matrix.distances.len() != matrix.durations.len()) {
        return Err("distance and duration collections have different length".into());
    }

    if costs.iter().any(|matrix| (matrix.distances.len() as f64).sqrt().round() as usize != size) {
        return Err("distance lengths don't match".into());
    }

    if costs.iter().any(|matrix| (matrix.durations.len() as f64).sqrt().round() as usize != size) {
        return Err("duration lengths don't match".into());
    }

    Ok(if costs.iter().any(|costs| costs.timestamp.is_some()) {
        Arc::new(TimeAwareMatrixTransportCost::new(costs, size, fallback)?)
    } else {
        Arc::new(TimeAgnosticMatrixTransportCost::new(costs, size, fallback)?)
    })
}

/// A time agnostic matrix routing costs.
struct TimeAgnosticMatrixTransportCost<T: TransportFallback> {
    durations: Vec<Vec<Duration>>,
    distances: Vec<Vec<Distance>>,
    size: usize,
    fallback: T,
}

impl<T: TransportFallback> TimeAgnosticMatrixTransportCost<T> {
    /// Creates an instance of `TimeAgnosticMatrixTransportCost`.
    pub fn new(costs: Vec<MatrixData>, size: usize, fallback: T) -> Result<Self, GenericError> {
        let mut costs = costs;
        costs.sort_by(|a, b| a.index.cmp(&b.index));

        if costs.iter().any(|costs| costs.timestamp.is_some()) {
            return Err("time aware routing".into());
        }

        if (0..).zip(costs.iter().map(|c| &c.index)).any(|(a, &b)| a != b) {
            return Err("duplicate profiles can be passed only for time aware routing".into());
        }

        let (durations, distances) = costs.into_iter().fold((vec![], vec![]), |mut acc, data| {
            acc.0.push(data.durations);
            acc.1.push(data.distances);

            acc
        });

        Ok(Self { durations, distances, size, fallback })
    }
}

impl<T: TransportFallback> TransportCost for TimeAgnosticMatrixTransportCost<T> {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        self.durations
            .get(profile.index)
            .unwrap()
            .get(from * self.size + to)
            .copied()
            .unwrap_or_else(|| self.fallback.duration(profile, from, to))
            * profile.scale
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        self.distances
            .get(profile.index)
            .unwrap()
            .get(from * self.size + to)
            .copied()
            .unwrap_or_else(|| self.fallback.distance(profile, from, to))
    }

    fn duration(&self, route: &Route, from: Location, to: Location, _: TravelTime) -> Duration {
        self.duration_approx(&route.actor.vehicle.profile, from, to)
    }

    fn distance(&self, route: &Route, from: Location, to: Location, _: TravelTime) -> Distance {
        self.distance_approx(&route.actor.vehicle.profile, from, to)
    }
}

/// A time aware matrix costs.
struct TimeAwareMatrixTransportCost<T: TransportFallback> {
    costs: HashMap<usize, (Vec<u64>, Vec<MatrixData>)>,
    size: usize,
    fallback: T,
}

impl<T: TransportFallback> TimeAwareMatrixTransportCost<T> {
    /// Creates an instance of `TimeAwareMatrixTransportCost`.
    fn new(costs: Vec<MatrixData>, size: usize, fallback: T) -> Result<Self, GenericError> {
        if costs.iter().any(|matrix| matrix.timestamp.is_none()) {
            return Err("time-aware routing requires all matrices to have timestamp".into());
        }

        let costs = costs.into_iter().collect_group_by_key(|matrix| matrix.index);

        if costs.iter().any(|(_, matrices)| matrices.len() == 1) {
            return Err("should not use time aware matrix routing with single matrix".into());
        }

        let costs = costs
            .into_iter()
            .map(|(profile, mut matrices)| {
                matrices.sort_by(|a, b| (a.timestamp.unwrap() as u64).cmp(&(b.timestamp.unwrap() as u64)));
                let timestamps = matrices.iter().map(|matrix| matrix.timestamp.unwrap() as u64).collect();

                (profile, (timestamps, matrices))
            })
            .collect();

        Ok(Self { costs, size, fallback })
    }

    fn interpolate_duration(
        &self,
        profile: &Profile,
        from: Location,
        to: Location,
        travel_time: TravelTime,
    ) -> Duration {
        let timestamp = match travel_time {
            TravelTime::Arrival(arrival) => arrival,
            TravelTime::Departure(departure) => departure,
        };

        let (timestamps, matrices) = self.costs.get(&profile.index).unwrap();
        let data_idx = from * self.size + to;

        let duration = match timestamps.binary_search(&(timestamp as u64)) {
            Ok(matrix_idx) => matrices.get(matrix_idx).unwrap().durations.get(data_idx).copied(),
            Err(0) => matrices.first().unwrap().durations.get(data_idx).copied(),
            Err(matrix_idx) if matrix_idx == matrices.len() => {
                matrices.last().unwrap().durations.get(data_idx).copied()
            }
            Err(matrix_idx) => {
                let left_matrix = matrices.get(matrix_idx - 1).unwrap();
                let right_matrix = matrices.get(matrix_idx).unwrap();

                matrices
                    .get(matrix_idx - 1)
                    .unwrap()
                    .durations
                    .get(data_idx)
                    .zip(matrices.get(matrix_idx).unwrap().durations.get(data_idx))
                    .map(|(&left_value, &right_value)| {
                        // perform linear interpolation
                        let ratio = (timestamp - left_matrix.timestamp.unwrap())
                            / (right_matrix.timestamp.unwrap() - left_matrix.timestamp.unwrap());

                        left_value + ratio * (right_value - left_value)
                    })
            }
        }
        .unwrap_or_else(|| self.fallback.duration(profile, from, to));

        duration * profile.scale
    }

    fn interpolate_distance(
        &self,
        profile: &Profile,
        from: Location,
        to: Location,
        travel_time: TravelTime,
    ) -> Distance {
        let timestamp = match travel_time {
            TravelTime::Arrival(arrival) => arrival,
            TravelTime::Departure(departure) => departure,
        };

        let (timestamps, matrices) = self.costs.get(&profile.index).unwrap();
        let data_idx = from * self.size + to;

        match timestamps.binary_search(&(timestamp as u64)) {
            Ok(matrix_idx) => matrices.get(matrix_idx).unwrap().distances.get(data_idx),
            Err(0) => matrices.first().unwrap().distances.get(data_idx),
            Err(matrix_idx) if matrix_idx == matrices.len() => matrices.last().unwrap().distances.get(data_idx),
            Err(matrix_idx) => matrices.get(matrix_idx - 1).unwrap().distances.get(data_idx),
        }
        .copied()
        .unwrap_or_else(|| self.fallback.distance(profile, from, to))
    }
}

impl<T: TransportFallback> TransportCost for TimeAwareMatrixTransportCost<T> {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        self.interpolate_duration(profile, from, to, TravelTime::Departure(0.))
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        self.interpolate_distance(profile, from, to, TravelTime::Departure(0.))
    }

    fn duration(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Duration {
        self.interpolate_duration(&route.actor.vehicle.profile, from, to, travel_time)
    }

    fn distance(&self, route: &Route, from: Location, to: Location, travel_time: TravelTime) -> Distance {
        self.interpolate_distance(&route.actor.vehicle.profile, from, to, travel_time)
    }
}
