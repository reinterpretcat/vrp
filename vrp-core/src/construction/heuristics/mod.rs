//! A generalized insertion heuristic implementation.
//!
//! # Design
//!

mod context;
pub use self::context::*;

mod evaluators;
pub use self::evaluators::*;

mod factories;

mod insertions;
pub use self::insertions::*;

mod metrics;
pub use self::metrics::*;

mod selectors;
pub use self::selectors::*;
