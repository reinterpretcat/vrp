extern crate serde_json;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    pub lat: f64,
    pub lng: f64,
}

impl Location {
    pub fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
    }
}

pub mod coord_index;

pub mod problem;
pub mod solution;
