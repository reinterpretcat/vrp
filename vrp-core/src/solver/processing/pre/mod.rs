//! Contains pre processing logic for the problem.

use crate::models::Problem;
use crate::utils::Environment;
use std::sync::Arc;

mod cluster_jobs;
pub use self::cluster_jobs::ClusterJobs;

/// A trait which specifies the logic to apply pre processing to problem.
pub trait PreProcessing {
    /// Applies pre processing to given problem.
    fn process(&self, problem: Arc<Problem>, environment: Arc<Environment>) -> Arc<Problem>;
}
