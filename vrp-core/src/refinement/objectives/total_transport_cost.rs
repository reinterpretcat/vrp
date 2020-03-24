#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use super::*;
use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::RefinementContext;
use crate::utils::VariationCoefficient;

/// An objective function which calculate total cost.
pub struct TotalTransportCost {
    cost_goal: Option<(f64, bool)>,
    variation_goal: Option<VariationCoefficient>,
    tolerance: Option<f64>,
}

impl Default for TotalTransportCost {
    fn default() -> Self {
        Self { cost_goal: None, variation_goal: None, tolerance: None }
    }
}

impl TotalTransportCost {
    pub fn new(cost_goal: Option<Cost>, variation_goal: Option<(usize, f64)>, tolerance: Option<f64>) -> Self {
        Self {
            cost_goal: cost_goal.map(|cost| (cost, true)),
            variation_goal: variation_goal
                .map(|(sample, threshold)| VariationCoefficient::new(sample, threshold, "cost_vc")),
            tolerance,
        }
    }

    fn get_actual_cost(&self, insertion_ctx: &InsertionContext) -> Cost {
        insertion_ctx.solution.routes.iter().fold(Cost::default(), |acc, rc| {
            let actor = &rc.route.actor;

            let start = rc.route.tour.start().unwrap();
            let problem = &insertion_ctx.problem;
            let initial = problem.activity.cost(actor, start, start.schedule.arrival);
            let initial = initial + actor.vehicle.costs.fixed + actor.driver.costs.fixed;

            acc + rc.route.tour.legs().fold(initial, |acc, (items, _)| {
                acc + match items {
                    [from, to] => {
                        problem.activity.cost(actor, to, to.schedule.arrival)
                            + problem.transport.cost(
                                actor,
                                from.place.location,
                                to.place.location,
                                from.schedule.departure,
                            )
                    }
                    [_] => 0.0,
                    _ => panic!("Unexpected route leg configuration."),
                }
            })
        })
    }
}

impl Objective for TotalTransportCost {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        Box::new(MeasurableObjectiveCost::new_with_tolerance(self.get_actual_cost(insertion_ctx), self.tolerance))
    }

    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool> {
        let actual_cost = self.get_actual_cost(insertion_ctx);

        check_value_variation_goals(refinement_ctx, actual_cost, &self.cost_goal, &self.variation_goal)
    }
}
