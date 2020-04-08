//! A core crate contains main buildings blocks for constructing heuristics and metaheuristic
//! to solve rich ***Vehicle Routing Problem***.
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod construction;
pub mod models;
pub mod refinement;
pub mod utils;
