#[cfg(test)]
#[path = "../../tests/unit/utils/variation_coefficient_test.rs"]
mod variation_coefficient_test;

use crate::refinement::{Individuum, RefinementContext};

/// Uses coefficient of variation as termination criteria.
pub struct VariationCoefficient {
    capacity: usize,
    threshold: f64,
    key: String,
}

impl VariationCoefficient {
    /// Creates a new instance of [`VariationCoefficient`].
    pub fn new(capacity: usize, threshold: f64, key: &str) -> Self {
        Self { capacity, threshold, key: key.to_string() }
    }

    /// Updates refinement_ctx and checks variation coefficient threshold.
    pub fn update_and_check(&self, refinement_ctx: &mut RefinementContext, individuum: &Individuum) -> bool {
        let costs = refinement_ctx
            .state
            .entry(self.key.clone())
            .or_insert_with(|| Box::new(vec![0.; self.capacity]))
            .downcast_mut::<Vec<f64>>()
            .unwrap();

        costs[refinement_ctx.generation % self.capacity] = individuum.1.value();

        refinement_ctx.generation >= (self.capacity - 1) && self.check_threshold(costs)
    }

    fn check_threshold(&self, costs: &Vec<f64>) -> bool {
        let sum: f64 = costs.iter().sum();
        let mean = sum / self.capacity as f64;
        let variance = self.calculate_variance(costs, mean);
        let sdev = variance.sqrt();
        let cv = sdev / mean;

        cv < self.threshold
    }

    fn calculate_variance(&self, costs: &Vec<f64>, mean: f64) -> f64 {
        let (first, second) = costs.iter().fold((0., 0.), |acc, v| {
            let dev = v - mean;
            (acc.0 + dev * dev, acc.1 + dev)
        });

        (first - (second * second / self.capacity as f64)) / (self.capacity as f64 - 1.)
    }
}
