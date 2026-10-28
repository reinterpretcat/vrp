//! A crate for solving Vehicle Routing Problem using default metaheuristic.
//!
//!
//! This crate provides ready-to-use functionality to solve rich ***Vehicle Routing Problem***.
//!
//! For more details check the following resources:
//!
//! - [`user guide`](https://reinterpretcat.github.io/vrp) describes how to use cli
//!   application built from this crate
//! - `vrp-core` crate implements default metaheuristic
//! - [`interop`] module exposes the solver to other languages

#![warn(missing_docs)]
#![deny(unsafe_code)] // NOTE: use deny instead forbid as we need allow unsafe code for c_interop
#![allow(clippy::items_after_test_module)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
mod helpers;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
#[path = "../tests/features/mod.rs"]
mod features;

pub use vrp_core as core;
pub use vrp_pragmatic as pragmatic;
#[cfg(feature = "scientific-format")]
pub use vrp_scientific as scientific;

pub mod extensions;
pub mod interop;

pub use crate::interop::json::{get_locations_serialized, get_solution_serialized};
