#[cfg(test)]
#[path = "../../../tests/unit/json/problem/deserializer_test.rs"]
mod deserializer_test;

extern crate serde_json;

use self::serde_json::Error;
use crate::json::Location;
use serde::Deserialize;
use std::io::{BufReader, Read};

// region Plan

/// Relation type.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum RelationType {
    /// Tour relation locks jobs to specific vehicle in any order.
    Tour,
    /// Flexible relation locks jobs in specific order allowing insertion of other jobs in between.
    Flexible,
    /// Sequence relation locks jobs in strict order, no insertions in between are allowed.
    Sequence,
}

/// Relation is the way to lock specific jobs to specific vehicles.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    /// Relation type.
    #[serde(rename(deserialize = "type"))]
    pub type_field: RelationType,
    /// List of job ids.
    pub jobs: Vec<String>,
    /// Vehicle id.
    pub vehicle_id: String,
    /// Vehicle shift index.
    pub shift_index: Option<usize>,
}

/// Defines specific job place.
#[derive(Clone, Deserialize, Debug)]
pub struct JobPlace {
    /// A list of job time windows with time specified in RFC3339 format.
    pub times: Option<Vec<Vec<String>>>,
    /// Job location.
    pub location: Location,
    /// Job duration (service time).
    pub duration: f64,
    /// An tag which will be propagated back within corresponding activity in solution.
    pub tag: Option<String>,
}

/// Specifies pickup and delivery places of the job.
/// At least one place should be specified. If only delivery specified, then vehicle is loaded with
/// job's demand at the start location. If only pickup specified, then loaded good is delivered to
/// the last location on the route. When both, pickup and delivery, are specified, then it is classical
/// pickup and delivery job.
#[derive(Clone, Deserialize, Debug)]
pub struct JobPlaces {
    /// Pickup place.
    pub pickup: Option<JobPlace>,
    /// Delivery place.
    pub delivery: Option<JobPlace>,
}

/// Specifies single job.
#[derive(Clone, Deserialize, Debug)]
pub struct Job {
    /// Job id.
    pub id: String,
    /// Job places.
    pub places: JobPlaces,
    /// Job demand.
    pub demand: Vec<i32>,
    /// Job skills.
    pub skills: Option<Vec<String>>,
}

/// Specifies a place for sub job.
#[derive(Clone, Deserialize, Debug)]
pub struct MultiJobPlace {
    /// A list of sub job time windows with time specified in RFC3339 format.
    pub times: Option<Vec<Vec<String>>>,
    /// Sub job location.
    pub location: Location,
    /// Sub job duration (service time).
    pub duration: f64,
    /// Sub job demand.
    pub demand: Vec<i32>,
    /// An tag which will be propagated back within corresponding activity in solution.
    pub tag: Option<String>,
}

/// Specifies pickups and deliveries places of multi job.
/// All of them should be completed or none of them. All pickups must be completed before any of deliveries.
#[derive(Clone, Deserialize, Debug)]
pub struct MultiJobPlaces {
    /// A list of pickups.
    pub pickups: Vec<MultiJobPlace>,
    /// A list of deliveries.
    pub deliveries: Vec<MultiJobPlace>,
}

/// Specifies multi job which has multiple child jobs.
#[derive(Clone, Deserialize, Debug)]
pub struct MultiJob {
    /// Multi job id.
    pub id: String,
    /// Multi job places.
    pub places: MultiJobPlaces,
    /// Multi job skills.
    pub skills: Option<Vec<String>>,
}

/// Job variant type.
#[derive(Clone, Deserialize, Debug)]
#[serde(untagged)]
pub enum JobVariant {
    /// Single job.
    Single(Job),
    /// Multi job.
    Multi(MultiJob),
}

/// A plan specifies work which has to be done.
#[derive(Clone, Deserialize, Debug)]
pub struct Plan {
    /// List of jobs.
    pub jobs: Vec<JobVariant>,
    /// List of relations between jobs and vehicles.
    pub relations: Option<Vec<Relation>>,
}

// endregion

// region Fleet

/// Specifies vehicle costs.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleCosts {
    /// Fixed is cost of vehicle usage per tour.
    pub fixed: Option<f64>,
    /// Cost per distance unit.
    pub distance: f64,
    /// Cost per time unit.
    pub time: f64,
}

/// Specifies vehicle place.
#[derive(Clone, Deserialize, Debug)]
pub struct VehiclePlace {
    /// Vehicle start or end time.
    pub time: String,
    /// Vehicle location.
    pub location: Location,
}

/// Specifies vehicle shift.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VehicleShift {
    /// Vehicle start place.
    pub start: VehiclePlace,
    /// Vehicle end place.
    pub end: Option<VehiclePlace>,
    /// Vehicle breaks.
    pub breaks: Option<Vec<VehicleBreak>>,
    /// Vehicle reloads which allows vehicle to return back to the depot (or any other place) in
    /// order to unload/load goods during single tour.
    pub reloads: Option<Vec<VehicleReload>>,
}

/// Vehicle reload.
pub type VehicleReload = JobPlace;

/// Vehicle limits.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VehicleLimits {
    /// Max traveling distance per shift/tour.
    pub max_distance: Option<f64>,
    /// Max time per shift/tour.
    pub shift_time: Option<f64>,
}

/// Vehicle break time variant.
#[derive(Clone, Deserialize, Debug)]
#[serde(untagged)]
pub enum VehicleBreakTime {
    /// Break time is defined by a list of time windows with time specified in RFC3339 format.
    TimeWindows(Vec<Vec<String>>),
    /// Break time is defined by max working (shift) time before break should happen.
    IntervalWindow(Vec<f64>),
}

/// Vehicle break.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleBreak {
    /// Break time.
    pub times: VehicleBreakTime,
    /// Break duration.
    pub duration: f64,
    /// Break location.
    pub location: Option<Location>,
}

/// Specifies a vehicle type.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleType {
    /// Vehicle type id.
    pub id: String,
    /// Vehicle profile name.
    pub profile: String,
    /// Vehicle costs.
    pub costs: VehicleCosts,
    /// Vehicle shifts.
    pub shifts: Vec<VehicleShift>,
    /// Vehicle capacity.
    pub capacity: Vec<i32>,
    /// Vehicle amount.
    pub amount: i32,
    /// Vehicle skills.
    pub skills: Option<Vec<String>>,
    /// Vehicle limits.
    pub limits: Option<VehicleLimits>,
}

/// Specifies routing profile.
#[derive(Clone, Deserialize, Debug)]
pub struct Profile {
    /// Profile name.
    pub name: String,
    /// Profile type.
    #[serde(rename(deserialize = "type"))]
    pub profile_type: String,
}

/// Specifies fleet.
#[derive(Clone, Deserialize, Debug)]
pub struct Fleet {
    /// Vehicle types.
    pub types: Vec<VehicleType>,
    /// Routing profiles.
    pub profiles: Vec<Profile>,
}

// endregion

// region Configuration

/// Specifies extra configuration.
#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    /// Features config.
    pub features: Option<Features>,
}

/// Specifies features config.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    /// Even distribution of the jobs across tours. By default, is off.
    pub even_distribution: Option<EvenDistribution>,
}

/// Configuration to tweak even distribution of the jobs across tours.
#[derive(Clone, Deserialize, Debug)]
pub struct EvenDistribution {
    /// Enable or disable.
    pub enabled: bool,
    /// A fraction of this cost is applied when jobs are assigned to the tour.
    pub extra_cost: Option<f64>,
}

// endregion

// region Common

/// A VRP problem definition.
#[derive(Clone, Deserialize, Debug)]
pub struct Problem {
    /// Problem plan: customers to serve.
    pub plan: Plan,
    /// Problem resources: vehicles to be used, routing info.
    pub fleet: Fleet,
    /// Extra configuration.
    pub config: Option<Config>,
}

/// A routing matrix.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    /// Number of unique locations.
    pub num_origins: i32,
    /// Number of unique locations.
    pub num_destinations: i32,
    /// Travel distances.
    pub travel_times: Vec<i64>,
    /// Travel durations.
    pub distances: Vec<i64>,
    /// Error codes to mark unreachable locations.
    pub error_codes: Option<Vec<i64>>,
}

// endregion

/// Deserializes problem in json format from [`BufReader`].
pub fn deserialize_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, Error> {
    serde_json::from_reader(reader)
}

/// Deserializes routing matrix in json format from [`BufReader`].
pub fn deserialize_matrix<R: Read>(reader: BufReader<R>) -> Result<Matrix, Error> {
    serde_json::from_reader(reader)
}
