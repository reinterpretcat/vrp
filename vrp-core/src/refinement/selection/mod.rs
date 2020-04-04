//! Specifies solution selection logic.

use crate::construction::heuristics::InsertionContext;
use crate::refinement::RefinementContext;

/// Provides the way to select solution for next iteration.
pub trait Selection {
    /// Selects solution from given `refinement_ctx`.
    fn select(&self, refinement_ctx: &mut RefinementContext) -> InsertionContext;
}

mod select_best;
pub use self::select_best::SelectBest;

mod select_random;
pub use self::select_random::SelectRandom;
