//! Contains post processing logic for solution.

use crate::construction::heuristics::InsertionContext;

mod advance_departure;
pub use self::advance_departure::AdvanceDeparture;

/// A trait which specifies the logic to apply post processing to solution.
pub trait PostProcessing {
    /// Applies post processing to given solution.
    fn process(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}
