#[cfg(test)]
#[path = "../../../tests/unit/format/problem/model_test.rs"]
mod model_test;

extern crate serde_json;

use crate::format::{FormatError, Location, MultiFormatError};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Error, Read, Write};

// region Plan

/// Relation type.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationType {
    /// Relation type which locks jobs to specific vehicle in any order.
    Any,
    /// Relation type which locks jobs in specific order allowing insertion of other jobs in between.
    Sequence,
    /// Relation type which locks jobs in strict order, no insertions in between are allowed.
    Strict,
}

/// Relation is the way to lock specific jobs to specific vehicles.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    /// Relation type.
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub type_field: RelationType,
    /// List of job ids.
    pub jobs: Vec<String>,
    /// Vehicle id.
    pub vehicle_id: String,
    /// Vehicle shift index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift_index: Option<usize>,
}

/// A job skills limitation for a vehicle.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSkills {
    /// Vehicle should have all of these skills defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<String>>,
    /// Vehicle should have at least one of these skills defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<String>>,
    /// Vehicle should have none of these skills defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<String>>,
}

/// Specifies a place for sub job.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct JobPlace {
    /// A job place location.
    pub location: Location,
    /// A job place duration (service time).
    pub duration: f64,
    /// A list of job place time windows with time specified in RFC3339 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<Vec<Vec<String>>>,
    /// A tag which will be propagated back within corresponding activity in solution.
    /// You can use it to identify used place in solution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Specifies a job task.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct JobTask {
    /// A list of possible places where given task can be performed.
    pub places: Vec<JobPlace>,
    /// Job place demand.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand: Option<Vec<i32>>,
    /// An order, bigger value - later assignment in the route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
}

/// A customer job model. Actual tasks of the job specified by list of pickups and deliveries
/// which follows these rules:
/// * all of them should be completed or none of them.
/// * all pickups must be completed before any of deliveries.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Job {
    /// A job id.
    pub id: String,

    /// A list of pickup tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pickups: Option<Vec<JobTask>>,

    /// A list of delivery tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliveries: Option<Vec<JobTask>>,

    /// A list of replacement tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacements: Option<Vec<JobTask>>,

    /// A list of service tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<JobTask>>,

    /// A job skills limitations for serving a job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<JobSkills>,

    /// Job value, bigger value - more chances for assignment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,

    /// Job group: jobs of the same group are assigned to the same tour or unassigned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// A compatibility group: jobs with different compatibility cannot be assigned to the same tour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
}

// region Clustering

/// Specifies clustering algorithm.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
pub enum Clustering {
    /// Vicinity clustering.
    #[serde(rename(deserialize = "vicinity", serialize = "vicinity"))]
    Vicinity {
        /// Specifies a vehicle profile used to calculate commute duration and distance between
        /// activities in the single stop.
        profile: VehicleProfile,
        /// Specifies threshold information.
        threshold: VicinityThresholdPolicy,
        /// Specifies visiting policy.
        visiting: VicinityVisitPolicy,
        /// Specifies service time policy.
        serving: VicinityServingPolicy,
        /// Specifies filtering policy.
        filtering: Option<VicinityFilteringPolicy>,
    },
}

/// Defines a various thresholds to control cluster size.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VicinityThresholdPolicy {
    /// Moving duration limit.
    pub duration: f64,
    /// Moving distance limit.
    pub distance: f64,
    /// Minimum shared time for jobs (non-inclusive).
    pub min_shared_time: Option<f64>,
    /// The smallest time window of the cluster after service time shrinking.
    pub smallest_time_window: Option<f64>,
    /// The maximum amount of jobs per cluster.
    pub max_jobs_per_cluster: Option<usize>,
}

/// Specifies cluster visiting policy.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VicinityVisitPolicy {
    /// It is required to return to the first job's location (cluster center) before visiting a next job.
    Return,
    /// Clustered jobs are visited one by one from the cluster center finishing in the end at the
    /// first job's location.
    Continue,
}

/// Specifies service time policy.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
pub enum VicinityServingPolicy {
    /// Keep original service time.
    #[serde(rename(deserialize = "original", serialize = "original"))]
    Original {
        /// Parking time.
        parking: f64,
    },
    /// Correct service time by some multiplier.
    #[serde(rename(deserialize = "multiplier", serialize = "multiplier"))]
    Multiplier {
        /// Multiplier value applied to original job's duration.
        value: f64,
        /// Parking time.
        parking: f64,
    },
    /// Use fixed value for all clustered jobs.
    #[serde(rename(deserialize = "fixed", serialize = "fixed"))]
    Fixed {
        /// Fixed value used for all jobs in the cluster.
        value: f64,
        /// Parking time.
        parking: f64,
    },
}

/// Specifies filtering policy for vicinity clustering.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VicinityFilteringPolicy {
    /// Ids of the jobs which cannot be used within clustering.
    pub exclude_job_ids: Vec<String>,
}

// endregion

/// A plan specifies work which has to be done.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Plan {
    /// List of jobs.
    pub jobs: Vec<Job>,

    /// List of relations between jobs and vehicles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<Relation>>,

    /// Specifies clustering parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clustering: Option<Clustering>,
}

// endregion

// region Fleet

/// Specifies vehicle costs.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct VehicleCosts {
    /// Fixed is cost of vehicle usage per tour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed: Option<f64>,

    /// Cost per distance unit.
    pub distance: f64,

    /// Cost per time unit.
    pub time: f64,
}

/// Specifies vehicle shift start.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct ShiftStart {
    /// Earliest possible departure date time in RFC3339 format.
    pub earliest: String,

    /// Latest possible departure date time in RFC3339 format. If omitted, departure time
    /// theoretically can be shifted till arrival. Set this value, if you want to limit
    /// departure time optimization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,

    /// Shift start location.
    pub location: Location,
}

/// Specifies vehicle shift end.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct ShiftEnd {
    /// Earliest possible arrival date time in RFC3339 format.
    /// At the moment, not supported, reserved for future.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest: Option<String>,

    /// Latest possible arrival date time in RFC3339 format.
    pub latest: String,

    /// Shift end location.
    pub location: Location,
}

/// Specifies vehicle shift.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct VehicleShift {
    /// Vehicle shift start.
    pub start: ShiftStart,

    /// Vehicle shift end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ShiftEnd>,

    /// Vehicle breaks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaks: Option<Vec<VehicleBreak>>,

    /// Vehicle reloads which allows vehicle to visit place where goods can be loaded or
    /// unloaded during single tour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reloads: Option<Vec<VehicleReload>>,

    /// Vehicle recharge stations information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recharges: Option<VehicleRecharges>,
}

/// Specifies a place where vehicle can load or unload cargo.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleReload {
    /// A place location.
    pub location: Location,

    /// A total loading/reloading duration (service time).
    pub duration: f64,

    /// A list of time windows with time specified in RFC3339 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub times: Option<Vec<Vec<String>>>,

    /// A tag which will be propagated back within corresponding activity in solution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// A shared reload resource id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
}

/// Specifies vehicle recharge stations data.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleRecharges {
    /// Maximum traveled distance before recharge station has to be visited.
    pub max_distance: f64,

    /// Specifies list of recharge station. Each can be visited only once.
    pub stations: Vec<VehicleRechargeStation>,
}

/// Specifies type alias for vehicle recharge station.
pub type VehicleRechargeStation = JobPlace;

/// Vehicle limits.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleLimits {
    /// Max traveling distance per shift/tour.
    /// No distance restrictions when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<f64>,

    /// Max duration per tour.
    /// No time restrictions when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(alias = "shiftTime")]
    pub max_duration: Option<f64>,

    /// Max amount job activities.
    /// No job activities restrictions when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tour_size: Option<usize>,
}

/// Vehicle optional break time variant.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum VehicleOptionalBreakTime {
    /// Break time is defined by a time window with time specified in RFC3339 format.
    TimeWindow(Vec<String>),
    /// Break time is defined by a time offset range.
    TimeOffset(Vec<f64>),
}

/// Vehicle required break time variant.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum VehicleRequiredBreakTime {
    /// Break time is defined by exact time in RFC3339 format.
    /// Break should be taken not earlier and not later than time range specified.
    ExactTime {
        /// Start of the range.
        earliest: String,
        /// End of the range.
        latest: String,
    },
    /// Break time is defined by amount of seconds since driving time.
    /// Break should be taken not earlier and not later than time range specified.
    OffsetTime {
        /// Start of the range.
        earliest: f64,
        /// End of the range.
        latest: f64,
    },
}

/// Vehicle break place.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct VehicleOptionalBreakPlace {
    /// Break duration.
    pub duration: f64,
    /// Break location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// A tag which will be propagated back within corresponding activity in solution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Vehicle break policy.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VehicleOptionalBreakPolicy {
    /// Allows to skip break if actual tour schedule doesn't intersect with vehicle time window.
    SkipIfNoIntersection,
    /// Allows to skip break if vehicle arrives before break's time window end.
    SkipIfArrivalBeforeEnd,
}

/// Specifies a vehicle break.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum VehicleBreak {
    /// An optional break which is more flexible, but might be not assigned.
    Optional {
        /// Break time.
        time: VehicleOptionalBreakTime,
        /// Vehicle break places.
        places: Vec<VehicleOptionalBreakPlace>,
        /// Specifies vehicle break policy.
        policy: Option<VehicleOptionalBreakPolicy>,
    },
    /// A break which has to be assigned. It is less flexible than optional break, but has strong
    /// assignment guarantee.
    Required {
        /// Break time.
        time: VehicleRequiredBreakTime,
        /// Break duration.
        duration: f64,
    },
}

/// Specifies a vehicle type.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleType {
    /// Vehicle type id.
    pub type_id: String,

    /// Concrete vehicle ids.
    pub vehicle_ids: Vec<String>,

    /// Vehicle profile.
    pub profile: VehicleProfile,

    /// Vehicle costs.
    pub costs: VehicleCosts,

    /// Vehicle shifts.
    pub shifts: Vec<VehicleShift>,

    /// Vehicle capacity.
    pub capacity: Vec<i32>,

    /// Vehicle skills.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,

    /// Vehicle limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<VehicleLimits>,
}

/// Specifies a vehicle profile.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct VehicleProfile {
    /// Routing matrix profile name.
    pub matrix: String,

    /// Traveling duration scale factor.
    /// Default value is 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

/// Specifies routing matrix profile.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct MatrixProfile {
    /// Profile name.
    pub name: String,

    /// Approximation speed (meters per second). Used only when routing matrix is not specified.
    /// Default value is 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

/// Specifies vehicle resource type.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(tag = "type")]
pub enum VehicleResource {
    /// A shared reload resource.
    #[serde(rename(deserialize = "reload", serialize = "reload"))]
    Reload {
        /// Resource id.
        id: String,
        /// A total resource capacity.
        capacity: Vec<i32>,
    },
}

/// Specifies fleet.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Fleet {
    /// Vehicle types.
    pub vehicles: Vec<VehicleType>,

    /// Routing profiles.
    pub profiles: Vec<MatrixProfile>,

    /// Specifies vehicle resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<VehicleResource>>,
}

// endregion

// region Objective

/// Specifies objective function types.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Objective {
    /// An objective to minimize total cost as a linear combination of total time and distance.
    MinimizeCost,

    /// An objective to minimize total distance.
    MinimizeDistance,

    /// An objective to minimize total duration.
    MinimizeDuration,

    /// An objective to minimize total tour amount.
    MinimizeTours,

    /// An objective to maximize total tour amount.
    MaximizeTours,

    /// An objective to maximize value of served jobs.
    MaximizeValue {
        /// Specifies a weight of skipped breaks.
        #[serde(skip_serializing_if = "Option::is_none")]
        breaks: Option<f64>,
    },

    /// An objective to minimize number of unassigned jobs.
    MinimizeUnassigned {
        /// A skipped break weight to increase/decrease break is importance.
        /// Default is 1.
        #[serde(skip_serializing_if = "Option::is_none")]
        breaks: Option<f64>,
    },

    /// An objective to minimize sum of arrival times from all routes.
    MinimizeArrivalTime,

    /// An objective to balance max load across all tours.
    BalanceMaxLoad,

    /// An objective to balance activities across all tours.
    BalanceActivities,

    /// An objective to balance distance across all tours.
    BalanceDistance,

    /// An objective to balance duration across all tours.
    BalanceDuration,

    /// An objective to control how tours are built.
    CompactTour {
        /// Specifies radius of neighbourhood. Min is 1.
        job_radius: usize,
    },

    /// An objective to control order of job activities in the tour.
    TourOrder,

    /// An objective to prefer jobs to be served as soon as possible.
    FastService,

    /// A composite objective allows to define multiple competitive objectives at the same layer of hierarchy.
    Composite {
        /// An objective composition type.
        composition_type: CompositionType,
        /// Competitive objectives except `Composite` type (nesting is currently not supported).
        objectives: Vec<Objective>,
    },
}

/// An objective composition type specifies how competitive objective functions are compared among each other.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CompositionType {
    /// A sum composition type simply sums all objective values together.
    Sum,

    /// A weighted sum composition type uses linear combination of weights and the corresponding fitness values.
    WeightedSum {
        /// Individual weights. Size of vector must be the same as amount of objective functions.
        weights: Vec<f64>,
    },
}

// endregion

// region Common

/// A VRP problem definition.
#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Problem {
    /// Problem plan: customers to serve.
    pub plan: Plan,

    /// Problem resources: vehicles to be used, routing info.
    pub fleet: Fleet,

    /// Specifies objective functions in lexicographical order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objectives: Option<Vec<Objective>>,
}

/// A routing matrix.
#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    /// A name of profile.
    pub profile: Option<String>,

    /// A date in RFC3999 for which routing info is applicable.
    pub timestamp: Option<String>,

    /// Travel distances (used to be in seconds).
    #[serde(alias = "durations")]
    pub travel_times: Vec<i64>,

    /// Travel durations (use to be in meters).
    pub distances: Vec<i64>,

    /// Error codes to mark unreachable locations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_codes: Option<Vec<i64>>,
}

// endregion

impl Job {
    /// Returns iterator over all tasks.
    pub fn all_tasks_iter(&self) -> impl Iterator<Item = &JobTask> {
        self.pickups
            .iter()
            .chain(self.deliveries.iter())
            .chain(self.services.iter())
            .chain(self.replacements.iter())
            .flatten()
    }
}

/// Deserializes problem in json format from `BufReader`.
pub fn deserialize_problem<R: Read>(reader: BufReader<R>) -> Result<Problem, MultiFormatError> {
    serde_json::from_reader(reader).map_err(|err| {
        vec![FormatError::new(
            "E0000".to_string(),
            "cannot deserialize problem".to_string(),
            format!("check input json: '{err}'"),
        )]
        .into()
    })
}

/// Deserializes routing matrix in json format from `BufReader`.
pub fn deserialize_matrix<R: Read>(reader: BufReader<R>) -> Result<Matrix, MultiFormatError> {
    serde_json::from_reader(reader).map_err(|err| {
        vec![FormatError::new(
            "E0001".to_string(),
            "cannot deserialize matrix".to_string(),
            format!("check input json: '{err}'"),
        )]
        .into()
    })
}

/// Deserializes json list of locations from `BufReader`.
pub fn deserialize_locations<R: Read>(reader: BufReader<R>) -> Result<Vec<Location>, MultiFormatError> {
    serde_json::from_reader(reader).map_err(|err| {
        vec![FormatError::new(
            "E0000".to_string(),
            "cannot deserialize locations".to_string(),
            format!("check input json: '{err}'"),
        )]
        .into()
    })
}

/// Serializes `problem` in json from `writer`.
pub fn serialize_problem<W: Write>(problem: &Problem, writer: &mut BufWriter<W>) -> Result<(), Error> {
    serde_json::to_writer_pretty(writer, problem).map_err(Error::from)
}
