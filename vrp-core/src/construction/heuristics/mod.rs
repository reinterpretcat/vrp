//! A generalized insertion heuristic implementation.
//!
//! # Design
//!
//! Checks each insertion possibility in parallel.

mod evaluators;

mod insertions;
pub use self::insertions::*;
