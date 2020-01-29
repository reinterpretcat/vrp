#[cfg(test)]
#[path = "../../../tests/unit/refinement/termination/variation_coefficient_test.rs"]
mod variation_coefficient_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::termination::Termination;
use crate::refinement::RefinementContext;

/// Uses coefficient of variation as termination criteria.
pub struct VariationCoefficient {
    capacity: usize,
    threshold: f64,
    last_cost: Option<f64>,
    costs: Vec<f64>,
}

impl Termination for VariationCoefficient {
    fn is_termination(
        &mut self,
        refinement_ctx: &mut RefinementContext,
        solution: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool {
        // TODO do we need to consider penalties?

        if solution.2 {
            self.last_cost = Some(solution.1.actual);
        }

        let index = refinement_ctx.generation % self.capacity;

        self.costs[index] = solution.1.actual;

        refinement_ctx.generation >= (self.capacity - 1) && self.check_threshold()
    }
}

impl Default for VariationCoefficient {
    fn default() -> Self {
        Self::new(200, 0.05)
    }
}

impl VariationCoefficient {
    /// Creates a new instance of [`VariationCoefficient`].
    pub fn new(capacity: usize, threshold: f64) -> Self {
        let costs = vec![0.; capacity];
        Self { capacity, threshold, last_cost: None, costs }
    }

    fn check_threshold(&self) -> bool {
        let sum: f64 = self.costs.iter().sum();
        let mean = sum / self.capacity as f64;
        let variance = self.calculate_variance(mean);
        let sdev = variance.sqrt();
        let cv = sdev / mean;

        cv < self.threshold
    }

    fn calculate_variance(&self, mean: f64) -> f64 {
        let (first, second) = self.costs.iter().fold((0., 0.), |acc, v| {
            let dev = v - mean;
            (acc.0 + dev * dev, acc.1 + dev)
        });

        (first - (second * second / self.capacity as f64)) / (self.capacity as f64 - 1.)
    }
}
