use crate::json::Location;
use serde::Serialize;
use serde_json::Error;
use std::io::{BufWriter, Write};

/// Timing statistic.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Timing {
    /// Driving time.
    pub driving: i32,
    /// Serving time.
    pub serving: i32,
    /// Waiting time.
    pub waiting: i32,
    /// Break time.
    #[serde(rename(serialize = "break"))]
    pub break_time: i32,
}

/// Represents statistic.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Statistic {
    /// Total cost.
    pub cost: f64,
    /// Total distance.
    pub distance: i32,
    /// Total duration.
    pub duration: i32,
    /// Timing statistic.
    pub times: Timing,
}

/// Represents a schedule.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Schedule {
    /// Arrival time specified in RFC3339 format.
    pub arrival: String,
    /// Departure time specified in RFC3339 format.
    pub departure: String,
}

/// Represents time interval.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Interval {
    /// Start time specified in RFC3339 format.
    pub start: String,
    /// End time specified in RFC3339 format.
    pub end: String,
}

/// An activity is unit of work performed at some place.
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Job id.
    pub job_id: String,
    /// Activity type.
    #[serde(rename(serialize = "type"))]
    pub activity_type: String,
    /// Location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Active time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Interval>,
    /// Job tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "tag"))]
    pub job_tag: Option<String>,
}

/// A stop is a place where vehicle is supposed to be parked.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Stop {
    /// Stop location.
    pub location: Location,
    /// Stop schedule.
    pub time: Schedule,
    /// Distance traveled since departure from start.
    pub distance: i32,
    /// Vehicle load after departure from this stop.
    pub load: Vec<i32>,
    /// Activities performed at the stop.
    pub activities: Vec<Activity>,
}

/// A tour is list of stops with their activities performed by specific vehicle.
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tour {
    /// Vehicle id.
    pub vehicle_id: String,
    /// Vehicle type id.
    pub type_id: String,
    /// Shift index.
    pub shift_index: usize,
    /// List of stops.
    pub stops: Vec<Stop>,
    /// Tour statistic.
    pub statistic: Statistic,
}

/// Unassigned job reason.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct UnassignedJobReason {
    /// A reason code.
    pub code: i32,
    /// Description.
    pub description: String,
}

/// Unassigned job.
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedJob {
    /// Job id.
    pub job_id: String,
    /// Possible reasons.
    pub reasons: Vec<UnassignedJobReason>,
}

/// Defines iteration model.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Iteration {
    /// Iteration number.
    pub number: i32,
    /// Best known cost
    pub cost: f64,
    /// Elapsed time in seconds.
    pub timestamp: f64,
    /// Amount of tours
    pub tours: usize,
    /// Amount of unassigned jobs.
    pub unassinged: usize,
}

/// Contains extra information.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Extras {
    /// Stores information about iteration performance.
    pub performance: Vec<Iteration>,
}

/// A VRP solution.
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    /// A VRP problem id.
    pub problem_id: String,
    /// Total statistic.
    pub statistic: Statistic,
    /// List of tours.
    pub tours: Vec<Tour>,
    /// List of unassigned jobs.
    pub unassigned: Vec<UnassignedJob>,
    /// An extra information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<Extras>,
}

/// Serializes solution into json format.
pub fn serialize_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    serde_json::to_writer_pretty(writer, solution)
}
