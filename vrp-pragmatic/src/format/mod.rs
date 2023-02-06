//! This module defines logic to serialize/deserialize problem and routing matrix in pragmatic
//! format from json input and create and write pragmatic solution.
//!

extern crate serde_json;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::BufWriter;

use vrp_core::models::problem::ReservedTimesIndex;
use vrp_core::models::problem::{Fleet as CoreFleet, Job as CoreJob};
use vrp_core::models::Problem as CoreProblem;

mod coord_index;
pub use self::coord_index::CoordIndex;

pub mod problem;
pub mod solution;

/// Represents a location type.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Location {
    /// A location type represented by geocoordinate with latitude and longitude.
    Coordinate {
        /// Latitude.
        lat: f64,
        /// Longitude.
        lng: f64,
    },
    /// A location type represented by index reference in routing matrix.
    Reference {
        /// An index in routing matrix.
        index: usize,
    },
}

impl Location {
    /// Creates a new `[Location]` as coordinate.
    pub fn new_coordinate(lat: f64, lng: f64) -> Self {
        Self::Coordinate { lat, lng }
    }

    /// Creates a new `[Location]` as index reference.
    pub fn new_reference(index: usize) -> Self {
        Self::Reference { index }
    }

    /// Returns lat lng if location is coordinate, panics otherwise.
    pub fn to_lat_lng(&self) -> (f64, f64) {
        match self {
            Self::Coordinate { lat, lng } => (*lat, *lng),
            _ => unreachable!("expect coordinate"),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Location::Coordinate { lat, lng } => write!(f, "lat={lat}, lng={lng}"),
            Location::Reference { index } => write!(f, "index={index}"),
        }
    }
}

/// A format error.
#[derive(Clone, Debug, Serialize)]
pub struct FormatError {
    /// An error code in registry.
    pub code: String,
    /// A possible error cause.
    pub cause: String,
    /// An action to take in order to recover from error.
    pub action: String,
    /// A details about exception.
    pub details: Option<String>,
}

impl FormatError {
    /// Creates a new instance of `FormatError` action without details.
    pub fn new(code: String, cause: String, action: String) -> Self {
        Self { code, cause, action, details: None }
    }

    /// Creates a new instance of `FormatError` action.
    pub fn new_with_details(code: String, cause: String, action: String, details: String) -> Self {
        Self { code, cause, action, details: Some(details) }
    }

    /// Serializes error into json string.
    pub fn to_json(&self) -> String {
        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        serde_json::to_writer_pretty(writer, &self).unwrap();

        buffer
    }

    /// Formats multiple format errors into json string.
    pub fn format_many_to_json(errors: &[Self]) -> String {
        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        serde_json::to_writer_pretty(writer, errors).unwrap();

        buffer
    }

    /// Formats multiple format errors into string.
    pub fn format_many(errors: &[Self], separator: &str) -> String {
        errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join(separator)
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, cause: '{}', action: '{}'.", self.code, self.cause, self.action)
    }
}

const TIME_CONSTRAINT_CODE: i32 = 1;
const DISTANCE_LIMIT_CONSTRAINT_CODE: i32 = 2;
const DURATION_LIMIT_CONSTRAINT_CODE: i32 = 3;
const CAPACITY_CONSTRAINT_CODE: i32 = 4;
const BREAK_CONSTRAINT_CODE: i32 = 5;
const SKILL_CONSTRAINT_CODE: i32 = 6;
const LOCKING_CONSTRAINT_CODE: i32 = 7;
const REACHABLE_CONSTRAINT_CODE: i32 = 8;
const AREA_CONSTRAINT_CODE: i32 = 9;
const DISPATCH_CONSTRAINT_CODE: i32 = 10;
const TOUR_SIZE_CONSTRAINT_CODE: i32 = 11;
const TOUR_ORDER_CONSTRAINT_CODE: i32 = 12;
const GROUP_CONSTRAINT_CODE: i32 = 13;
const COMPATIBILITY_CONSTRAINT_CODE: i32 = 14;
const RELOAD_RESOURCE_CONSTRAINT_CODE: i32 = 15;

/// An job id to job index.
pub type JobIndex = HashMap<String, CoreJob>;

/// Gets job index from core problem definition.
pub fn get_job_index(problem: &CoreProblem) -> &JobIndex {
    problem
        .extras
        .get("job_index")
        .and_then(|s| s.downcast_ref::<JobIndex>())
        .unwrap_or_else(|| panic!("cannot get job index!"))
}

/// Gets coord index from core problem definition.
pub fn get_coord_index(problem: &CoreProblem) -> &CoordIndex {
    problem.extras.get("coord_index").and_then(|s| s.downcast_ref::<CoordIndex>()).expect("cannot get coord index!")
}

/// Gets reserved times index.
pub fn get_reserved_times_index(problem: &CoreProblem) -> &ReservedTimesIndex {
    problem
        .extras
        .get("reserved_times_index")
        .and_then(|s| s.downcast_ref::<ReservedTimesIndex>())
        .expect("cannot get reserved time index!")
}
