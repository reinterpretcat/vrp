use crate::format::{CoordIndex, Location};
use crate::{format_time, parse_time};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Error, Read, Write};
use vrp_core::models::common::{Duration, Timestamp};
use vrp_core::models::solution::Commute as DomainCommute;
use vrp_core::models::solution::CommuteInfo as DomainCommuteInfo;

/// Timing statistic.
#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct Timing {
    /// Driving time.
    pub driving: i64,
    /// Serving time.
    pub serving: i64,
    /// Waiting time.
    pub waiting: i64,
    /// Break time.
    #[serde(rename(serialize = "break", deserialize = "break"))]
    pub break_time: i64,
    /// Commuting time.
    #[serde(default = "i64::default")]
    pub commuting: i64,
    /// Parking time.
    #[serde(default = "i64::default")]
    pub parking: i64,
}

/// Represents statistic.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Statistic {
    /// Total cost.
    pub cost: f64,
    /// Total distance.
    pub distance: i64,
    /// Total duration.
    pub duration: i64,
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

/// Stores information about commuting to perform activity.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Commute {
    /// Commuting to the activity place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward: Option<CommuteInfo>,
    /// Commuting from the activity place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backward: Option<CommuteInfo>,
}

/// Stores information about commuting information in one direction.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct CommuteInfo {
    /// Commute location.
    pub location: Location,
    /// Travelled distance.
    pub distance: f64,
    /// Travel time.
    pub time: Interval,
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
    /// Activity time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Interval>,
    /// Job tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_tag: Option<String>,
    /// Commute information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commute: Option<Commute>,
}

/// A stop is a place where vehicle is supposed to be parked.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Stop {
    /// Stop location.
    pub location: Location,
    /// Stop schedule.
    pub time: Schedule,
    /// Distance traveled since departure from start.
    #[serde(default)]
    pub distance: i64,
    /// Vehicle load after departure from this stop.
    pub load: Vec<i32>,
    /// Parking time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parking: Option<Interval>,
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
    #[serde(default)]
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
    pub code: String,
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

/// Specifies a type of violation.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum Violation {
    /// A break assignment violation.
    #[serde(rename(deserialize = "break", serialize = "break"))]
    Break {
        /// An id of a vehicle break belong to.
        vehicle_id: String,
        /// Index of the shift.
        shift_index: usize,
    },
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
#[serde(rename_all = "camelCase")]
pub struct Generation {
    /// Generation sequence number.
    pub number: usize,
    /// Time since evolution started.
    pub timestamp: f64,
    /// Overall improvement ratio.
    pub i_all_ratio: f64,
    /// Improvement ratio last 1000 generations.
    pub i_1000_ratio: f64,
    /// True if this generation considered as improvement.
    pub is_improvement: bool,
    /// Population state.
    pub population: Population,
}

/// Keeps essential information about particular individual in population.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Individual {
    /// Solution cost difference from best individual.
    pub improvement: f64,
    /// Objectives fitness values.
    pub fitness: Vec<f64>,
}

/// Holds population state.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Population {
    /// Population individuals.
    pub individuals: Vec<Individual>,
}

/// Contains extra information.
#[derive(Clone, Deserialize, Serialize, PartialEq, Debug)]
pub struct Extras {
    /// A telemetry metrics.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unassigned: Option<Vec<UnassignedJob>>,

    /// List of constraint violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub violations: Option<Vec<Violation>>,

    /// An extra information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<Extras>,
}

/// Serializes solution into json format.
pub fn serialize_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    serde_json::to_writer_pretty(writer, solution).map_err(Error::from)
}

/// Deserializes solution from json format.
pub fn deserialize_solution<R: Read>(reader: BufReader<R>) -> Result<Solution, Error> {
    serde_json::from_reader(reader).map_err(Error::from)
}

impl Interval {
    /// Returns interval's duration.
    pub fn duration(&self) -> Duration {
        parse_time(&self.end) - parse_time(&self.start)
    }
}

impl Commute {
    /// Creates a new instance of `Commute`.
    pub fn new(commute: &DomainCommute, start: Timestamp, end: Timestamp, coord_index: &CoordIndex) -> Commute {
        let parse_info = |info: &DomainCommuteInfo, time: Timestamp| {
            if info.is_zero_distance() {
                None
            } else {
                Some(CommuteInfo {
                    location: coord_index.get_by_idx(info.location).expect("commute info has no location"),
                    distance: info.distance,
                    time: Interval { start: format_time(time), end: format_time(time + info.duration) },
                })
            }
        };

        Commute { forward: parse_info(&commute.forward, start), backward: parse_info(&commute.backward, end) }
    }

    /// Converts given commute object to core model.
    pub(crate) fn to_domain(&self, coord_index: &CoordIndex) -> DomainCommute {
        let parse_info = |info: &Option<CommuteInfo>| {
            info.as_ref().map_or(DomainCommuteInfo::default(), |info| {
                let start = parse_time(&info.time.start);
                let end = parse_time(&info.time.end);
                DomainCommuteInfo {
                    location: coord_index.get_by_loc(&info.location).expect("expect to have coordinate in commute"),
                    distance: info.distance,
                    duration: end - start,
                }
            })
        };

        DomainCommute { forward: parse_info(&self.forward), backward: parse_info(&self.backward) }
    }
}
