//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
mod helpers;

#[cfg(test)]
#[path = "../tests/checker/mod.rs"]
mod checker;

#[cfg(test)]
#[path = "../tests/generator/mod.rs"]
mod generator;

#[cfg(test)]
#[path = "../tests/features/mod.rs"]
mod features;

#[cfg(test)]
#[path = "../tests/slow/mod.rs"]
pub mod slow;

mod constraints;
mod extensions;
mod utils;
mod validation;

pub mod json;

use crate::json::coord_index::CoordIndex;
use crate::json::problem::Problem;
use crate::json::Location;
use chrono::{DateTime, ParseError, SecondsFormat, TimeZone, Utc};

/// Get lists of problem.
pub fn get_unique_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(&problem).unique()
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_time(time: &String) -> f64 {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &String) -> Result<f64, ParseError> {
    DateTime::parse_from_rfc3339(time).map(|time| time.timestamp() as f64)
}
