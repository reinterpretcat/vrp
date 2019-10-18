#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/penalize_unassigned_test.rs"]
mod penalize_unassigned_test;

use crate::models::common::{Cost, ObjectiveCost};
use crate::models::{Problem, Solution};
use crate::refinement::objectives::ObjectiveFunction;

pub struct PenalizeUnassigned {
    penalty: Cost,
}

impl PenalizeUnassigned {
    pub fn new(penalty: Cost) -> Self {
        Self { penalty }
    }
}

impl ObjectiveFunction for PenalizeUnassigned {
    fn estimate(&self, problem: &Problem, solution: &Solution) -> ObjectiveCost {
        let actual = solution.routes.iter().fold(Cost::default(), |acc, r| {
            let start = r.tour.start().unwrap();
            let initial = problem.activity.cost(&r.actor.vehicle, &r.actor.driver, start, start.schedule.arrival);
            let initial = initial + r.actor.vehicle.costs.fixed + r.actor.driver.costs.fixed;
            acc + r.tour.legs().fold(initial, |acc, (items, _)| {
                let (from, to) = match items {
                    [from, to] => (from, to),
                    _ => panic!("Unexpected route leg configuration."),
                };
                acc + problem.activity.cost(&r.actor.vehicle, &r.actor.driver, to, to.schedule.arrival)
                    + problem.transport.cost(
                        &r.actor.vehicle,
                        &r.actor.driver,
                        from.place.location,
                        to.place.location,
                        from.schedule.departure,
                    )
            })
        });

        let penalty = solution.unassigned.len() as f64 * self.penalty;

        ObjectiveCost { actual, penalty }
    }
}
