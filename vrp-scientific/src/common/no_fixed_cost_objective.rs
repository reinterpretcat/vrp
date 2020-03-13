use vrp_core::construction::states::InsertionContext;
use vrp_core::refinement::objectives::{MeasurableObjectiveCost, MultiObjective, Objective, ObjectiveCost};
use vrp_core::refinement::RefinementContext;

/// Estimates solution ignoring fixed cost.
pub struct NoFixedCostObjective {
    inner_objective: Box<dyn Objective + Send + Sync>,
}

impl Default for NoFixedCostObjective {
    fn default() -> Self {
        Self { inner_objective: Box::new(MultiObjective::default()) }
    }
}

impl Objective for NoFixedCostObjective {
    fn estimate(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Box<dyn ObjectiveCost + Send + Sync> {
        let cost = self.inner_objective.estimate(refinement_ctx, insertion_ctx);

        let fixed = insertion_ctx
            .solution
            .routes
            .iter()
            .fold(0.0, |acc, r| acc + r.route.actor.driver.costs.fixed + r.route.actor.vehicle.costs.fixed);

        Box::new(MeasurableObjectiveCost::new(cost.value() - fixed))
    }
}
