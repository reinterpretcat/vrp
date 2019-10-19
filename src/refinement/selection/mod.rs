use crate::construction::states::InsertionContext;
use crate::refinement::RefinementContext;

/// Provides the way to select solution for next iteration.
pub trait Selection {
    fn select(&self, refinement_ctx: &RefinementContext) -> InsertionContext;
}

mod select_best;
pub use self::select_best::SelectBest;
