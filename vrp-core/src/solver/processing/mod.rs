//! Contains pre and post processing logic.

use crate::construction::heuristics::InsertionContext;
use crate::models::Problem;
use rosomaxa::utils::Environment;
use std::sync::Arc;

mod advance_departure;
pub use self::advance_departure::AdvanceDeparture;

mod unassignment_reason;
pub use self::unassignment_reason::UnassignmentReason;

mod vicinity_clustering;
pub use self::vicinity_clustering::{VicinityClustering, VicinityDimension};

/// A trait which specifies the logic to apply pre/post processing to problem/solution.
pub trait Processing {
    /// Applies pre processing to given problem.
    fn pre_process(&self, problem: Arc<Problem>, environment: Arc<Environment>) -> Arc<Problem>;

    /// Applies post processing to given solution.
    fn post_process(&self, insertion_ctx: InsertionContext) -> InsertionContext;
}

/// Provides the way to run multiple processors one by one on problem/solution.
pub struct CompositeProcessing {
    processors: Vec<Arc<dyn Processing + Send + Sync>>,
}

impl CompositeProcessing {
    /// Creates an instance of `CompositeProcessing`.
    pub fn new(processors: Vec<Arc<dyn Processing + Send + Sync>>) -> Self {
        Self { processors }
    }
}

impl Processing for CompositeProcessing {
    fn pre_process(&self, problem: Arc<Problem>, environment: Arc<Environment>) -> Arc<Problem> {
        self.processors.iter().fold(problem, |problem, processor| processor.pre_process(problem, environment.clone()))
    }

    fn post_process(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        self.processors
            .iter()
            .rev()
            .fold(insertion_ctx, |insertion_ctx, processor| processor.post_process(insertion_ctx))
    }
}
