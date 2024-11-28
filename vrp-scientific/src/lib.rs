//! Scientific crate contains logic to read scientific problems used to benchmark different
//! VRP related algorithms.
//!
//!
//! # Supported formats
//!
//! - **solomon**: see [Solomon benchmark](https://www.sintef.no/projectweb/top/vrptw/solomon-benchmark)
//! - **lilim**: see [Li&Lim benchmark](https://www.sintef.no/projectweb/top/pdptw/li-lim-benchmark)
//! - **tsplib** subset of TSPLIB95 format

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub(crate) mod helpers;

#[cfg(test)]
#[path = "../tests/integration/known_problems_test.rs"]
mod known_problems_test;

pub use vrp_core as core;

pub mod common;
pub mod lilim;
pub mod solomon;
pub mod tsplib;
