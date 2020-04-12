//! This module defines logic to serialize/deserialize problem and routing matrix in pragmatic
//! format from json input and create and write pragmatic solution.
//!

extern crate serde_json;
use serde::{Deserialize, Serialize};
use std::io::BufWriter;

/// A location type represented by latitude and longitude.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
}

impl Location {
    /// Creates new `[Location]`.
    pub fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
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

    /// Serializes error into json.
    pub fn to_json(&self) -> String {
        let mut buffer = String::new();
        let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
        serde_json::to_writer_pretty(writer, &self).unwrap();

        buffer
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
const SKILLS_CONSTRAINT_CODE: i32 = 6;
const LOCKING_CONSTRAINT_CODE: i32 = 7;
const REACHABLE_CONSTRAINT_CODE: i32 = 8;
const PRIORITY_CONSTRAINT_CODE: i32 = 9;

pub mod coord_index;

pub mod problem;
pub mod solution;
