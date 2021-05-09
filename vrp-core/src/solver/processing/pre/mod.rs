//! Contains pre processing logic for the problem.

use crate::models::Problem;

/// A trait which specifies the logic to apply pre processing to problem.
pub trait PreProcessing {
    /// Applies pre processing to given problem.
    fn process(&self, problem: Problem) -> Problem;
}
