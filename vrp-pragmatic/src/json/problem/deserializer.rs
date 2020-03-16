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
    /// Relation type which  locks jobs to specific vehicle in any order.
    Any,
    /// Relation type which  locks jobs in specific order allowing insertion of other jobs in between.
    Sequence,
    /// Relation type which locks jobs in strict order, no insertions in between are allowed.
    Strict,
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

/// Specifies a place for sub job.
#[derive(Clone, Deserialize, Debug)]
pub struct JobPlace {
    /// A job place location.
    pub location: Location,
    /// A job place duration (service time).
    pub duration: f64,
    /// A list of job place time windows with time specified in RFC3339 format.
    pub times: Option<Vec<Vec<String>>>,
}

/// Specifies a job task.
#[derive(Clone, Deserialize, Debug)]
pub struct JobTask {
    /// A list of possible places where given task can be performed.
    pub places: Vec<JobPlace>,
    /// Job place demand.
    pub demand: Option<Vec<i32>>,
    /// An tag which will be propagated back within corresponding activity in solution.
    pub tag: Option<String>,
}

/// A customer job model. Actual tasks of the job specified by list of pickups and deliveries
/// which follows these rules:
/// * all of them should be completed or none of them.
/// * all pickups must be completed before any of deliveries.
#[derive(Clone, Deserialize, Debug)]
pub struct Job {
    /// A job id.
    pub id: String,
    /// A list of pickup tasks.
    pub pickups: Option<Vec<JobTask>>,
    /// A list of delivery tasks.
    pub deliveries: Option<Vec<JobTask>>,
    /// A list of replacement tasks.
    pub replacements: Option<Vec<JobTask>>,
    /// A list of service tasks.
    pub services: Option<Vec<JobTask>>,
    /// Job priority, bigger value - less important.
    pub priority: Option<i32>,
    /// A set of skills required to serve a job.
    pub skills: Option<Vec<String>>,
}

/// A plan specifies work which has to be done.
#[derive(Clone, Deserialize, Debug)]
pub struct Plan {
    /// List of jobs.
    pub jobs: Vec<Job>,
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

/// Specifies a place for reload.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleReload {
    /// A reload location.
    pub location: Location,
    /// A reload duration (service time).
    pub duration: f64,
    /// A list of reload time windows with time specified in RFC3339 format.
    pub times: Option<Vec<Vec<String>>>,
    /// An tag which will be propagated back within corresponding activity in solution.
    pub tag: Option<String>,
}

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
    /// Break time is defined by a time window with time specified in RFC3339 format.
    TimeWindow(Vec<String>),
    /// Break time is defined by a time offset range.
    TimeOffset(Vec<f64>),
}

/// Vehicle break.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleBreak {
    /// Break time.
    pub time: VehicleBreakTime,
    /// Break duration.
    pub duration: f64,
    /// Break locations.
    pub locations: Option<Vec<Location>>,
}

/// Specifies a vehicle type.
#[derive(Clone, Deserialize, Debug)]
pub struct VehicleType {
    /// Vehicle type id.
    pub type_id: String,
    /// Concrete vehicle ids.
    pub vehicle_ids: Vec<String>,
    /// Vehicle profile name.
    pub profile: String,
    /// Vehicle costs.
    pub costs: VehicleCosts,
    /// Vehicle shifts.
    pub shifts: Vec<VehicleShift>,
    /// Vehicle capacity.
    pub capacity: Vec<i32>,
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
    pub vehicles: Vec<VehicleType>,
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
    /// Tweaks priority weight. Default value is 100.
    pub priority: Option<Priority>,
}

/// Configuration to tweak even distribution of the jobs across tours.
#[derive(Clone, Deserialize, Debug)]
pub struct Priority {
    /// A cost for formula: `extra_cost = (priority - 1) * weight_cost`.
    pub weight_cost: f64,
}

// endregion

// region Objective

/// Specifies a group of objective functions.
#[derive(Clone, Deserialize, Debug)]
pub struct Objectives {
    /// A list of primary objective functions. An accepted solution should not
    /// be worse of any of these.
    pub primary: Vec<Objective>,
    /// A list of secondary objective functions. An accepted solution can be worse
    /// by the secondary objective if it improves the primary one.
    pub secondary: Option<Vec<Objective>>,
}

/// Specifies objective function types.
#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Objective {
    /// An objective to minimize total cost.
    MinimizeCost {
        /// A termination criteria parameters.
        termination: Option<TerminationCriteria<f64>>,
    },
    /// An objective to minimize total tour amount.
    MinimizeTours {
        /// A termination criteria parameters.
        termination: Option<TerminationCriteria<usize>>,
    },
    /// An objective to minimize amount of unassigned jobs.
    MinimizeUnassignedJobs {
        /// A termination criteria parameters.
        termination: Option<TerminationCriteria<usize>>,
    },
    /// An objective to balance max load across all tours.
    BalanceMaxLoad {
        /// A relative load in single tour before balancing takes place.
        threshold: Option<f64>,
    },
    /// An objective to balance activities across all tours.
    BalanceActivities {
        /// A minimum amount of activities in single tour before balancing takes place.
        threshold: Option<usize>,
    },
}

/// Specifies termination criteria options.
#[derive(Clone, Deserialize, Debug)]
pub struct TerminationCriteria<T> {
    /// An absolute value threshold.
    pub value: Option<T>,
    /// A variation coefficient threshold.
    pub variation: Option<VariationCoefficient>,
}

/// Specifies parameters for variation coefficient calculations.
#[derive(Clone, Deserialize, Debug)]
pub struct VariationCoefficient {
    /// A sample size of refinement generations.
    pub sample: usize,
    /// A variation coefficient in percents.
    pub variation: f64,
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
    /// Specifies objective functions.
    pub objectives: Option<Objectives>,
    /// Extra configuration.
    pub config: Option<Config>,
}

/// A routing matrix.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
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
