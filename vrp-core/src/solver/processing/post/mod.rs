//! Contains post processing logic for solution.

use crate::construction::heuristics::InsertionContext;
use std::sync::Arc;

mod advance_departure;
pub use self::advance_departure::AdvanceDeparture;

mod unassignment_reason;
pub use self::unassignment_reason::UnassignmentReason;

mod uncluster_jobs;
pub use self::uncluster_jobs::UnclusterJobs;

/// A trait which specifies the logic to apply post processing to solution.
pub trait PostProcessing {
    /// Applies post processing to given solution.
    fn process(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// Provides the way to run multiple post processors one by one on the same solution.
pub struct CompositePostProcessing {
    post_processors: Vec<Arc<dyn PostProcessing + Send + Sync>>,
}

impl CompositePostProcessing {
    /// Creates an instance of `CompositePostProcessing`.
    pub fn new(post_processors: Vec<Arc<dyn PostProcessing + Send + Sync>>) -> Self {
        Self { post_processors }
    }
}

impl PostProcessing for CompositePostProcessing {
    fn process(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        self.post_processors
            .iter()
            .fold(insertion_ctx, |insertion_ctx, post_processor| post_processor.process(insertion_ctx))
    }
}
