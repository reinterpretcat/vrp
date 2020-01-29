use vrp_core::construction::states::InsertionContext;
use vrp_core::models::common::{Cost, ObjectiveCost};
use vrp_core::refinement::objectives::{Objective, PenalizeUnassigned};
use vrp_core::refinement::RefinementContext;

/// Estimates solution ignoring fixed cost.
pub struct NoFixedCostObjective {
    inner_objective: Box<dyn Objective + Send + Sync>,
}

impl NoFixedCostObjective {
    pub fn new(unassigned_penalty: Cost) -> Self {
        Self { inner_objective: Box::new(PenalizeUnassigned::new(unassigned_penalty)) }
    }
}

impl Default for NoFixedCostObjective {
    fn default() -> Self {
        Self { inner_objective: Box::new(PenalizeUnassigned::default()) }
    }
}

impl Objective for NoFixedCostObjective {
    fn estimate(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCost {
        let cost = self.inner_objective.estimate(refinement_ctx, insertion_ctx);

        let fixed = insertion_ctx
            .solution
            .routes
            .iter()
            .fold(0.0, |acc, r| acc + r.route.actor.driver.costs.fixed + r.route.actor.vehicle.costs.fixed);

        ObjectiveCost::new(cost.actual - fixed, cost.penalty)
    }
}
