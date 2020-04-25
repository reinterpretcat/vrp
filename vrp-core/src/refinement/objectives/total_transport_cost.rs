#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use super::*;
use crate::utils::compare_floats;

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
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(self.fitness(a), self.fitness(b))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        self.fitness(a) - self.fitness(b)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        self.get_actual_cost(solution)
    }
}
