use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;

/// Encapsulates objective function behaviour.
pub trait Objective {
    /// Estimates solution cost for given problem.
    fn estimate(&self, insertion_ctx: &InsertionContext) -> ObjectiveCost;
}

mod penalize_unassigned;
pub use self::penalize_unassigned::PenalizeUnassigned;
