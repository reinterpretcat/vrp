#[cfg(test)]
#[path = "../../../tests/unit/models/problem/costs_test.rs"]
mod costs_test;

use crate::construction::heuristics::InsertionContext;
use crate::models::common::*;
use crate::models::problem::{Actor, TargetObjective};
use crate::models::solution::Activity;
use crate::solver::objectives::{TotalCost, TotalRoutes, TotalUnassignedJobs};
use hashbrown::HashMap;
use rand::prelude::SliceRandom;
use rosomaxa::algorithms::nsga2::dominance_order;
use rosomaxa::population::Shuffled;
use rosomaxa::prelude::*;
use rosomaxa::utils::CollectGroupBy;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

/// A hierarchical multi objective for vehicle routing problem.
pub struct ProblemObjective {
    objectives: Vec<Vec<TargetObjective>>,
}

impl ProblemObjective {
    /// Creates an instance of `InsertionObjective`.
    pub fn new(objectives: Vec<Vec<TargetObjective>>) -> Self {
        Self { objectives }
    }
}

impl Objective for ProblemObjective {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        unwrap_from_result(self.objectives.iter().try_fold(Ordering::Equal, |_, objectives| {
            match dominance_order(a, b, objectives) {
                Ordering::Equal => Ok(Ordering::Equal),
                order => Err(order),
            }
        }))
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unreachable!()
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.solution.get_total_cost()
    }
}

impl MultiObjective for ProblemObjective {
    fn objectives<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a (dyn Objective<Solution = Self::Solution> + Send + Sync)> + 'a> {
        Box::new(self.objectives.iter().flatten().map(|o| o.as_ref()))
    }
}

impl HeuristicObjective for ProblemObjective {}

impl Shuffled for ProblemObjective {
    /// Returns a new instance of `ObjectiveCost` with shuffled objectives.
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self {
        let mut objectives = self.objectives.clone();

        objectives.shuffle(&mut random.get_rng());

        Self { objectives }
    }
}

impl Default for ProblemObjective {
    fn default() -> Self {
        Self::new(vec![
            vec![Arc::new(TotalUnassignedJobs::default())],
            vec![Arc::new(TotalRoutes::default())],
            vec![TotalCost::minimize()],
        ])
    }
}

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
    fn cost(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Cost {
        let waiting = if activity.place.time.start > arrival { activity.place.time.start - arrival } else { 0. };
        let service = activity.place.duration;

        waiting * (actor.driver.costs.per_waiting_time + actor.vehicle.costs.per_waiting_time)
            + service * (actor.driver.costs.per_service_time + actor.vehicle.costs.per_service_time)
    }

    /// Estimates departure time for activity and actor at given arrival time.
    fn estimate_departure(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp;

    /// Estimates arrival time for activity and actor at given departure time.
    fn estimate_arrival(&self, actor: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp;
}

/// An actor independent activity costs.
#[derive(Default)]
pub struct SimpleActivityCost {}

impl ActivityCost for SimpleActivityCost {
    fn estimate_departure(&self, _: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp {
        arrival.max(activity.place.time.start) + activity.place.duration
    }

    fn estimate_arrival(&self, _: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp {
        activity.place.time.end.min(departure - activity.place.duration)
    }
}

/// Specifies a function which returns an extra reserved time for given actor and time window
/// which will be considered by specific costs.
type ReservedTimeFunc = Arc<dyn Fn(&Actor, &TimeWindow) -> Option<TimeWindow> + Send + Sync>;

/// Provides way to calculate activity costs which might contain reserved time.
pub struct DynamicActivityCost {
    reserved_time_func: ReservedTimeFunc,
}

impl DynamicActivityCost {
    /// Creates a new instance of `DynamicActivityCost` with given reserved time function.
    pub fn new(reserved_times: HashMap<Arc<Actor>, Vec<TimeWindow>>) -> Result<Self, String> {
        Ok(Self { reserved_time_func: create_reserved_time_func(reserved_times)? })
    }
}

impl ActivityCost for DynamicActivityCost {
    fn estimate_departure(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp {
        let activity_start = arrival.max(activity.place.time.start);
        let departure = activity_start + activity.place.duration;
        let schedule = TimeWindow::new(arrival, departure);

        self.reserved_time_func.deref()(actor, &schedule).map_or(departure, |reserved_time: TimeWindow| {
            assert!(reserved_time.intersects(&schedule));

            let time_window = &activity.place.time;

            let extra_duration = if reserved_time.start < time_window.start {
                let waiting_time = TimeWindow::new(arrival, time_window.start);
                let overlapping = waiting_time.overlapping(&reserved_time).map(|tw| tw.duration()).unwrap_or(0.);

                reserved_time.duration() - overlapping
            } else {
                reserved_time.duration()
            };

            // NOTE: do not allow to start or restart work after break finished
            if activity_start + extra_duration > activity.place.time.end {
                f64::MAX
            } else {
                departure + extra_duration
            }
        })
    }

    fn estimate_arrival(&self, actor: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp {
        let arrival = activity.place.time.end.min(departure - activity.place.duration);
        let schedule = TimeWindow::new(arrival, departure);

        self.reserved_time_func.deref()(actor, &schedule).map_or(arrival, |reserved_time: TimeWindow| {
            // TODO consider overlapping break with waiting time?
            arrival - reserved_time.duration()
        })
    }
}

/// Provides the way to get routing information for specific locations and actor.
pub trait TransportCost {
    /// Returns time-dependent transport cost between two locations for given actor.
    fn cost(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Cost {
        let distance = self.distance(actor, from, to, travel_time);
        let duration = self.duration(actor, from, to, travel_time);

        distance * (actor.driver.costs.per_distance + actor.vehicle.costs.per_distance)
            + duration * (actor.driver.costs.per_driving_time + actor.vehicle.costs.per_driving_time)
    }

    /// Returns time-independent travel duration between locations specific for given profile.
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration;

    /// Returns time-independent travel distance between locations specific for given profile.
    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance;

    /// Returns time-dependent travel duration between locations specific for given actor.
    fn duration(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Duration;

    /// Returns time-dependent travel distance between locations specific for given actor.
    fn distance(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Distance;
}

/// Provides way to calculate transport costs which might contain reserved time.
pub struct DynamicTransportCost {
    reserved_time_func: ReservedTimeFunc,
    inner: Box<dyn TransportCost + Send + Sync>,
}

impl DynamicTransportCost {
    /// Creates a new instance of `DynamicTransportCost`.
    pub fn new(
        reserved_times: HashMap<Arc<Actor>, Vec<TimeWindow>>,
        inner: Box<dyn TransportCost + Send + Sync>,
    ) -> Result<Self, String> {
        Ok(Self { reserved_time_func: create_reserved_time_func(reserved_times)?, inner })
    }
}

impl TransportCost for DynamicTransportCost {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        self.inner.duration_approx(profile, from, to)
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        self.inner.distance_approx(profile, from, to)
    }

    fn duration(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Duration {
        let duration = self.inner.duration(actor, from, to, travel_time);

        let time_window = match travel_time {
            TravelTime::Arrival(arrival) => TimeWindow::new(arrival - duration, arrival),
            TravelTime::Departure(departure) => TimeWindow::new(departure, departure + duration),
        };

        self.reserved_time_func.deref()(actor, &time_window)
            .map_or(duration, |reserved_time: TimeWindow| duration + reserved_time.duration())
    }

    fn distance(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Distance {
        self.inner.distance(actor, from, to, travel_time)
    }
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

/// Creates time agnostic or time aware routing costs based on matrix data passed.
pub fn create_matrix_transport_cost(costs: Vec<MatrixData>) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
    if costs.is_empty() {
        return Err("no matrix data found".to_string());
    }

    let size = (costs.first().unwrap().durations.len() as f64).sqrt().round() as usize;

    if costs.iter().any(|matrix| matrix.distances.len() != matrix.durations.len()) {
        return Err("distance and duration collections have different length".to_string());
    }

    if costs.iter().any(|matrix| (matrix.distances.len() as f64).sqrt().round() as usize != size) {
        return Err("distance lengths don't match".to_string());
    }

    if costs.iter().any(|matrix| (matrix.durations.len() as f64).sqrt().round() as usize != size) {
        return Err("duration lengths don't match".to_string());
    }

    Ok(if costs.iter().any(|costs| costs.timestamp.is_some()) {
        Arc::new(TimeAwareMatrixTransportCost::new(costs, size)?)
    } else {
        Arc::new(TimeAgnosticMatrixTransportCost::new(costs, size)?)
    })
}

/// A time agnostic matrix routing costs.
struct TimeAgnosticMatrixTransportCost {
    durations: Vec<Vec<Duration>>,
    distances: Vec<Vec<Distance>>,
    size: usize,
}

impl TimeAgnosticMatrixTransportCost {
    /// Creates an instance of `TimeAgnosticMatrixTransportCost`.
    pub fn new(costs: Vec<MatrixData>, size: usize) -> Result<Self, String> {
        let mut costs = costs;
        costs.sort_by(|a, b| a.index.cmp(&b.index));

        if costs.iter().any(|costs| costs.timestamp.is_some()) {
            return Err("time aware routing".to_string());
        }

        if (0..).zip(costs.iter().map(|c| &c.index)).any(|(a, &b)| a != b) {
            return Err("duplicate profiles can be passed only for time aware routing".to_string());
        }

        let (durations, distances) = costs.into_iter().fold((vec![], vec![]), |mut acc, data| {
            acc.0.push(data.durations);
            acc.1.push(data.distances);

            acc
        });

        Ok(Self { durations, distances, size })
    }
}

impl TransportCost for TimeAgnosticMatrixTransportCost {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        *self.durations.get(profile.index).unwrap().get(from * self.size + to).unwrap() * profile.scale
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        *self.distances.get(profile.index).unwrap().get(from * self.size + to).unwrap()
    }

    fn duration(&self, actor: &Actor, from: Location, to: Location, _: TravelTime) -> Duration {
        self.duration_approx(&actor.vehicle.profile, from, to)
    }

    fn distance(&self, actor: &Actor, from: Location, to: Location, _: TravelTime) -> Distance {
        self.distance_approx(&actor.vehicle.profile, from, to)
    }
}

/// A time aware matrix costs.
struct TimeAwareMatrixTransportCost {
    costs: HashMap<usize, (Vec<u64>, Vec<MatrixData>)>,
    size: usize,
}

impl TimeAwareMatrixTransportCost {
    /// Creates an instance of `TimeAwareMatrixTransportCost`.
    fn new(costs: Vec<MatrixData>, size: usize) -> Result<Self, String> {
        if costs.iter().any(|matrix| matrix.timestamp.is_none()) {
            return Err("time-aware routing requires all matrices to have timestamp".to_string());
        }

        let costs = costs.into_iter().collect_group_by_key(|matrix| matrix.index);

        if costs.iter().any(|(_, matrices)| matrices.len() == 1) {
            return Err("should not use time aware matrix routing with single matrix".to_string());
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

        profile.scale
            * match timestamps.binary_search(&(timestamp as u64)) {
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
            Ok(matrix_idx) => *matrices.get(matrix_idx).unwrap().distances.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == 0 => *matrices.first().unwrap().distances.get(data_idx).unwrap(),
            Err(matrix_idx) if matrix_idx == matrices.len() => {
                *matrices.last().unwrap().distances.get(data_idx).unwrap()
            }
            Err(matrix_idx) => *matrices.get(matrix_idx - 1).unwrap().distances.get(data_idx).unwrap(),
        }
    }
}

impl TransportCost for TimeAwareMatrixTransportCost {
    fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
        self.interpolate_duration(profile, from, to, TravelTime::Departure(0.))
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        self.interpolate_distance(profile, from, to, TravelTime::Departure(0.))
    }

    fn duration(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Duration {
        self.interpolate_duration(&actor.vehicle.profile, from, to, travel_time)
    }

    fn distance(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Distance {
        self.interpolate_distance(&actor.vehicle.profile, from, to, travel_time)
    }
}

fn create_reserved_time_func(reserved_times: HashMap<Arc<Actor>, Vec<TimeWindow>>) -> Result<ReservedTimeFunc, String> {
    if reserved_times.is_empty() {
        return Ok(Arc::new(|_, _| None));
    }

    let reserved_times =
        reserved_times.into_iter().try_fold(HashMap::<_, (Vec<_>, Vec<_>)>::new(), |mut acc, (actor, mut times)| {
            times.sort_by(|a, b| compare_floats(a.start, b.start));
            let has_no_intersections =
                times.windows(2).all(|pair| if let [a, b] = pair { !a.intersects(b) } else { false });

            if has_no_intersections {
                let (indices, intervals): (Vec<_>, Vec<_>) = times.into_iter().map(|tw| (tw.start as u64, tw)).unzip();
                acc.insert(actor, (indices, intervals));

                Ok(acc)
            } else {
                Err("reserved times have intersections".to_string())
            }
        })?;

    Ok(Arc::new(move |actor: &Actor, time_window: &TimeWindow| {
        reserved_times
            .get(actor)
            .and_then(|(indices, intervals)| {
                match indices.binary_search(&(time_window.start as u64)) {
                    Ok(idx) => intervals.get(idx),
                    Err(idx) => (idx.max(1) - 1..=idx) // NOTE left (earliest) wins
                        .map(|idx| intervals.get(idx))
                        .find(|reserved_time| {
                            reserved_time.map_or(false, |reserved_time| {
                                // NOTE use exclusive intersection
                                compare_floats(time_window.start, reserved_time.end) == Ordering::Less
                                    && compare_floats(reserved_time.start, time_window.end) == Ordering::Less
                            })
                        })
                        .flatten(),
                }
            })
            .cloned()
    }))
}
