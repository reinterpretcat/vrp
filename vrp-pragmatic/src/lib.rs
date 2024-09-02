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

mod utils;

pub mod checker;
pub mod format;
pub mod validation;

use crate::format::problem::Problem;
use crate::format::{CoordIndex, Location};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use vrp_core::models::common::Timestamp;
use vrp_core::prelude::GenericError;

/// Get lists of unique locations in the problem. Use it to request routing matrix from outside.
/// NOTE: it includes all locations of all types, so you might need to filter it if types are mixed.
pub fn get_unique_locations(problem: &Problem) -> Vec<Location> {
    CoordIndex::new(problem).unique()
}

fn format_time(time: Timestamp) -> String {
    // TODO avoid using implicitly unwrap
    OffsetDateTime::from_unix_timestamp(time).map(|time| time.format(&Rfc3339).unwrap()).unwrap()
}

fn parse_time(time: &str) -> Timestamp {
    parse_time_safe(time).unwrap()
}

fn parse_time_safe(time: &str) -> Result<Timestamp, GenericError> {
    OffsetDateTime::parse(time, &Rfc3339)
        .map(|time| time.unix_timestamp() as Timestamp)
        .map_err(|err| format!("cannot parse date: {err}").into())
}
