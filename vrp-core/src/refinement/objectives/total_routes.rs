use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCost};
use crate::refinement::RefinementContext;

/// An objective function which counts total amount of routes.
pub struct TotalRoutes {}

impl Default for TotalRoutes {
    fn default() -> Self {
        Self {}
    }
}

impl Objective for TotalRoutes {
    fn estimate(
        &self,
        _: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Box<dyn ObjectiveCost + Send + Sync> {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.routes.len() as Cost))
    }
}
