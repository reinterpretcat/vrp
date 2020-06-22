//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.

#![warn(missing_docs)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
mod helpers;

#[cfg(test)]
#[path = "../tests/generator/mod.rs"]
mod generator;

#[cfg(test)]
#[path = "../tests/features/mod.rs"]
mod features;

#[cfg(test)]
#[path = "../tests/slow/mod.rs"]
pub mod slow;

pub use vrp_core as core;

mod constraints;
mod extensions;
mod utils;

pub mod checker;
pub mod format;
pub mod validation;

use crate::format::problem::Problem;
use crate::format::{CoordIndex, Location};
use chrono::{DateTime, ParseError, SecondsFormat, TimeZone, Utc};

/// Get lists of problem.
pub fn get_unique_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(&problem).unique()
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_time(time: &str) -> f64 {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &str) -> Result<f64, ParseError> {
    DateTime::parse_from_rfc3339(time).map(|time| time.timestamp() as f64)
}
