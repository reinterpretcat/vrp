#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/penalize_unassigned_test.rs"]
mod penalize_unassigned_test;

use crate::construction::states::InsertionContext;
use crate::models::common::{Cost, ObjectiveCost};
use crate::refinement::objectives::Objective;

pub struct PenalizeUnassigned {
    penalty: Cost,
}

impl PenalizeUnassigned {
    pub fn new(penalty: Cost) -> Self {
        Self { penalty }
    }
}

impl Default for PenalizeUnassigned {
    fn default() -> Self {
        Self::new(1E6)
    }
}

impl Objective for PenalizeUnassigned {
    fn estimate(&self, insertion_ctx: &InsertionContext) -> ObjectiveCost {
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

        let penalty = insertion_ctx.solution.unassigned.len() as f64 * self.penalty;

        ObjectiveCost { actual, penalty }
    }
}
