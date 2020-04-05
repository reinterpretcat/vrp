//! This module defines logic to serialize/deserialize problem and routing matrix in pragmatic
//! format from json input and create and write pragmatic solution.
//!

extern crate serde_json;
use serde::{Deserialize, Serialize};

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
