#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use super::*;
use crate::models::common::Objective;
use crate::utils::compare_floats;

/// An objective function which calculate total cost.
pub struct TotalTransportCost {}

impl Default for TotalTransportCost {
    fn default() -> Self {
        Self {}
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
        solution.solution.get_total_cost()
    }
}
