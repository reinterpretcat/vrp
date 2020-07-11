use crate::format::Location;
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::io::{BufReader, BufWriter, Read, Write};

/// Timing statistic.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Timing {
    /// Driving time.
    pub driving: i32,
    /// Serving time.
    pub serving: i32,
    /// Waiting time.
    pub waiting: i32,
    /// Break time.
    #[serde(rename(serialize = "break", deserialize = "break"))]
    pub break_time: i32,
}

/// Represents statistic.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
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
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Schedule {
    /// Arrival time specified in RFC3339 format.
    pub arrival: String,
    /// Departure time specified in RFC3339 format.
    pub departure: String,
}

/// Represents time interval.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Interval {
    /// Start time specified in RFC3339 format.
    pub start: String,
    /// End time specified in RFC3339 format.
    pub end: String,
}

/// An activity is unit of work performed at some place.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Job id.
    pub job_id: String,
    /// Activity type.
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub activity_type: String,
    /// Location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Active time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Interval>,
    /// Job tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_tag: Option<String>,
}

/// A stop is a place where vehicle is supposed to be parked.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
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
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
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
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct UnassignedJobReason {
    /// A reason code.
    pub code: i32,
    /// Description.
    pub description: String,
}

/// Unassigned job.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedJob {
    /// Job id.
    pub job_id: String,
    /// Possible reasons.
    pub reasons: Vec<UnassignedJobReason>,
}

/// Encapsulates different measurements regarding algorithm evaluation.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Metrics {
    /// Total algorithm duration.
    pub duration: usize,
    /// Total amount of generations.
    pub generations: usize,
    /// Speed: generations per second.
    pub speed: f64,
    /// Evolution progress.
    pub evolution: Vec<Generation>,
}

/// Represents information about generation.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Generation {
    /// Generation sequence number.
    pub number: usize,
    /// Time since evolution started.
    pub timestamp: f64,
    /// Population state.
    pub population: Vec<Individual>,
}

/// Keeps essential information about particular individual in population.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Individual {
    /// Total amount of tours.
    pub tours: usize,
    /// Total amount of unassigned jobs.
    pub unassigned: usize,
    /// Solution cost.
    pub cost: f64,
    /// Solution cost difference from best individual.
    pub improvement: f64,
    /// Objectives fitness values.
    pub fitness: Vec<f64>,
}

/// Contains extra information.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Extras {
    /// A telemetry metrics.
    pub metrics: Option<Metrics>,
}

/// A VRP solution.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
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

/// Deserializes solution from json format.
pub fn deserialize_solution<R: Read>(reader: BufReader<R>) -> Result<Solution, Error> {
    serde_json::from_reader(reader)
}
