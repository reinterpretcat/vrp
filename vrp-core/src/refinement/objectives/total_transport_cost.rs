#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCostType};
use crate::refinement::RefinementContext;

/// An objective function which calculate total cost.
pub struct TotalTransportCost {
    goal: Option<f64>,
}

impl Default for TotalTransportCost {
    fn default() -> Self {
        Self { goal: None }
    }
}

impl TotalTransportCost {
    pub fn new(desired_cost: Cost) -> Self {
        Self { goal: Some(desired_cost) }
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
        Box::new(MeasurableObjectiveCost::new(self.get_actual_cost(insertion_ctx)))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> Option<bool> {
        self.goal.map(|cost| cost <= self.get_actual_cost(insertion_ctx)).or(None)
    }
}
