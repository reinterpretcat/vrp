use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCost};
use crate::refinement::RefinementContext;

/// An objective function which counts total amount of unassigned jobs.
pub struct TotalUnassignedJobs {}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self {}
    }
}

impl Objective for TotalUnassignedJobs {
    fn estimate(
        &self,
        _: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Box<dyn ObjectiveCost + Send + Sync> {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.unassigned.len() as Cost))
    }
}
