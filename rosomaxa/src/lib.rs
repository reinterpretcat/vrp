//! This crate exposes a generalized hyper heuristics and some helper functionality which can be
//! used to build a solver for optimization problems.

#![warn(missing_docs)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod algorithms;
pub mod heuristics;
pub mod utils;

pub mod prelude;
