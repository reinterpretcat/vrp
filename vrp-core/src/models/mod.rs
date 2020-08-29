//! A collection of models to represent problem and solution in Vehicle Routing Problem domain.

pub(crate) const OP_START_MSG: &str = "Optional start is not yet implemented.";

pub mod common;

mod domain;
pub use self::domain::*;

/// TODO avoid it in production code
#[doc(hidden)]
pub mod examples;

pub mod problem;
pub mod solution;
