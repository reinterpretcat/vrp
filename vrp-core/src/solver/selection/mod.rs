//! Contains offspring selection algorithms.

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

mod naive_selection;
pub use self::naive_selection::NaiveSelection;

/// A trait which specifies evolution selection behavior.
pub trait Selection {
    /// Selects parent from population based on refinement process state.
    fn select_parents(&self, refinement_ctx: &RefinementContext) -> Vec<InsertionContext>;
}
