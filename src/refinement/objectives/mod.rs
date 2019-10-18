use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};

/// Encapsulates objective function behaviour.
pub trait Objective {
    /// Estimates solution cost for given problem.
    fn estimate(&self, problem: &Problem, solution: &Solution) -> ObjectiveCost;
}

mod penalize_unassigned;
pub use self::penalize_unassigned::PenalizeUnassigned;
