//! A generalized insertion heuristic implementation.
//!
//! # Design
//!
//! Checks each insertion possibility in parallel.

mod evaluators;
pub use self::evaluators::InsertionPosition;

mod insertions;
pub use self::insertions::*;
