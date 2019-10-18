#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/penalize_unassigned_test.rs"]
mod penalize_unassigned_test;

use crate::construction::states::InsertionContext;
use crate::models::common::{Cost, ObjectiveCost};
use crate::models::{Problem, Solution};
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
            let route = rc.route.read().unwrap();
            let actor = &route.actor;

            let start = route.tour.start().unwrap();
            let problem = &insertion_ctx.problem;
            let initial = problem.activity.cost(&actor.vehicle, &actor.driver, start, start.schedule.arrival);
            let initial = initial + actor.vehicle.costs.fixed + actor.driver.costs.fixed;

            acc + route.tour.legs().fold(initial, |acc, (items, _)| {
                let (from, to) = match items {
                    [from, to] => (from, to),
                    _ => panic!("Unexpected route leg configuration."),
                };
                acc + problem.activity.cost(&actor.vehicle, &actor.driver, to, to.schedule.arrival)
                    + problem.transport.cost(
                        &actor.vehicle,
                        &actor.driver,
                        from.place.location,
                        to.place.location,
                        from.schedule.departure,
                    )
            })
        });

        let penalty = insertion_ctx.solution.unassigned.len() as f64 * self.penalty;

        ObjectiveCost { actual, penalty }
    }
}
