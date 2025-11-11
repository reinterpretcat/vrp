//! Pragmatic crates aims to solve real world VRP variations allowing users to specify their problems
//! via simple **pragmatic** json format.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

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
use vrp_core::models::common::Timestamp;

mod utils;

pub mod checker;
pub mod format;
pub mod validation;

use crate::format::problem::Problem;
use crate::format::{CoordIndex, Location};
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, Time, UtcOffset};
use vrp_core::prelude::{Float, GenericError};

/// Get lists of unique locations in the problem. Use it to request routing matrix from outside.
/// NOTE: it includes all locations of all types, so you might need to filter it if types are mixed.
pub fn get_unique_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(problem).unique()
}

// vrp_core's Timestamps are f64s that could go far beyond what unix timestamps support
const MIN_TIMESTAMP: i64 = OffsetDateTime::new_in_offset(Date::MIN, Time::MIDNIGHT, UtcOffset::UTC).unix_timestamp();
const MAX_TIMESTAMP: i64 = OffsetDateTime::new_in_offset(Date::MAX, Time::MAX, UtcOffset::UTC).unix_timestamp();

fn format_time(time: Timestamp) -> String {
    let time: i64 = (time as i64).clamp(MIN_TIMESTAMP, MAX_TIMESTAMP);
    // TODO avoid using implicitly unwrap
    // (a priori the above clamping should prevent any potential failure here...)
    let ts = OffsetDateTime::from_unix_timestamp(time).expect("Could not convert value to timestamp");
    return ts.format(&Rfc3339).expect("Error formatting timestamp to time");
}

fn parse_time(time: &str) -> Float {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &str) -> Result<Float, GenericError> {
    OffsetDateTime::parse(time, &Rfc3339)
        .map(|time| time.unix_timestamp() as Float)
        .map_err(|err| format!("cannot parse date: {err}").into())
}
