//! A generalized insertion heuristic implementation.
//!
//! # Design
//!

mod cache;
use self::cache::*;

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

/// A key to store insertion evaluation cache.
const INSERTION_CACHE: i32 = 1;
