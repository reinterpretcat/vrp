//! A core crate contains main buildings blocks for constructing heuristics and metaheuristic
//! to solve rich ***Vehicle Routing Problem***.
//!
//! # Key points
//!
//! ## Constructive heuristic
//!
//! ## Metaheuristic
//!
//! # How to use
//!
//! ## The simplest way
//!
//! ## Tweak metaheuristic
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod algorithms;
pub mod construction;
pub mod models;
pub mod solver;
pub mod utils;
