//! Contains pre processing logic for the problem.

use crate::models::Problem;
use std::sync::Arc;

/// A trait which specifies the logic to apply pre processing to problem.
pub trait PreProcessing {
    /// Applies pre processing to given problem.
    fn process(&self, problem: Arc<Problem>) -> Arc<Problem>;
}
