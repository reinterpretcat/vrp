//! A collection of models to represent problem and solution in Vehicle Routing Problem domain.

pub(crate) const OP_START_MSG: &str = "Optional start is not yet implemented.";

mod domain;
pub use self::domain::*;

mod goal;
pub use self::goal::*;

pub mod common;
#[doc(hidden)]
pub mod examples;
pub mod problem;
pub mod solution;
