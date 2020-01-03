//! This module defines logic to serialize/deserialize problem and routing matrix in pragmatic
//! format from json input and create and write pragmatic solution.
//!
//! Check child modules for problem and solution definitions.

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

pub mod coord_index;

pub mod problem;
pub mod solution;
