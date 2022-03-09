#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/minimize_arrival_time_test.rs"]
mod minimize_arrival_time_test;

use super::*;
use rosomaxa::prelude::*;

/// An objective function which prefers solution with less total arrival time.
#[derive(Default)]
pub struct MinimizeArrivalTime {}

impl Objective for MinimizeArrivalTime {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        if solution.solution.routes.is_empty() {
            0.
        } else {
            let total: f64 = solution
                .solution
                .routes
                .iter()
                .filter_map(|route_ctx| route_ctx.route.tour.end())
                .map(|end| end.schedule.arrival)
                .sum();

            total / solution.solution.routes.len() as f64
        }
    }
}
