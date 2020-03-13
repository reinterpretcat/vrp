#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCost};
use crate::refinement::RefinementContext;

/// An objective function which calculate total cost.
pub struct TotalTransportCost {}

impl Default for TotalTransportCost {
    fn default() -> Self {
        Self {}
    }
}

impl Objective for TotalTransportCost {
    fn estimate(
        &self,
        _: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Box<dyn ObjectiveCost + Send + Sync> {
        let actual = insertion_ctx.solution.routes.iter().fold(Cost::default(), |acc, rc| {
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
        });

        Box::new(MeasurableObjectiveCost::new(actual))
    }
}
