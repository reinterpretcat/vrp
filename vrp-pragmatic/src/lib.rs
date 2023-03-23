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
#[allow(clippy::needless_update)]
mod features;

#[cfg(test)]
#[path = "../tests/discovery/mod.rs"]
pub mod discovery;

#[cfg(test)]
#[path = "../tests/regression/mod.rs"]
pub mod regression;

pub use vrp_core as core;

mod utils;

pub mod checker;
pub mod construction;
pub mod format;
pub mod validation;

use crate::format::problem::Problem;
use crate::format::{CoordIndex, Location};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Get lists of problem.
pub fn get_unique_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(problem).unique()
}

fn format_time(time: f64) -> String {
    // TODO avoid using implicitly unwrap
    OffsetDateTime::from_unix_timestamp(time as i64).map(|time| time.format(&Rfc3339).unwrap()).unwrap()
}

fn parse_time(time: &str) -> f64 {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &str) -> Result<f64, String> {
    OffsetDateTime::parse(time, &Rfc3339)
        .map(|time| time.unix_timestamp() as f64)
        .map_err(|err| format!("cannot parse date: {err}"))
}
