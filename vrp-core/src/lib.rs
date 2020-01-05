//! Core crate contains a main buildings blocks for metaheuristic to solve variations of ***Vehicle Routing Problem***.
//!
//! For more details please refer to [docs](index.html)

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod construction;
pub mod models;
pub mod refinement;
pub mod utils;
