//! Scientific crate contains logic to read scientific problems used to benchmark different
//! VRP related algorithms.
//!
//!
//! # Supported formats
//!
//! * Solomon
//! * LiLim

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

#[cfg(test)]
#[path = "../tests/integration/known_problems_test.rs"]
mod known_problems_test;

pub mod common;
pub mod lilim;
pub mod solomon;
mod utils;
