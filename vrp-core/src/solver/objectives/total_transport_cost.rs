#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_transport_cost_test.rs"]
mod total_transport_cost_test;

use super::*;
use crate::models::common::Objective;
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
        insertion_ctx.solution.get_actual_cost()
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
