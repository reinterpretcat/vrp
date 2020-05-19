//! A collection of models to represent problem and solution in Vehicle Routing Problem domain.

pub mod common;

mod domain;
pub use self::domain::*;

/// TODO avoid it in production code
#[doc(hidden)]
pub mod examples;

pub mod problem;
pub mod solution;
