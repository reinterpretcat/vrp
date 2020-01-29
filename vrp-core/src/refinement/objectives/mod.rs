//! Specifies objective functions.

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::RefinementContext;

/// Encapsulates objective function behaviour.
pub trait Objective {
    /// Estimates cost for given problem and solution.
    fn estimate(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCost;
}

mod penalize_unassigned;
pub use self::penalize_unassigned::PenalizeUnassigned;
