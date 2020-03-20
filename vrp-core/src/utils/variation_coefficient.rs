#[cfg(test)]
#[path = "../../tests/unit/utils/variation_coefficient_test.rs"]
mod variation_coefficient_test;

use crate::models::common::Cost;
use crate::refinement::RefinementContext;
use crate::utils::get_cv;

/// Uses coefficient of variation as termination criteria.
pub struct VariationCoefficient {
    sample: usize,
    threshold: f64,
    key: String,
}

impl VariationCoefficient {
    /// Creates a new instance of [`VariationCoefficient`].
    pub fn new(sample: usize, threshold: f64, key: &str) -> Self {
        Self { sample, threshold, key: key.to_string() }
    }

    /// Updates refinement_ctx and checks variation coefficient threshold.
    pub fn update_and_check(&self, refinement_ctx: &mut RefinementContext, cost: Cost) -> bool {
        let costs = refinement_ctx
            .state
            .entry(self.key.clone())
            .or_insert_with(|| Box::new(vec![0.; self.sample]))
            .downcast_mut::<Vec<f64>>()
            .unwrap();

        costs[refinement_ctx.generation % self.sample] = cost;

        refinement_ctx.generation >= (self.sample - 1) && self.check_threshold(costs)
    }

    fn check_threshold(&self, costs: &Vec<f64>) -> bool {
        get_cv(costs) < self.threshold
    }
}
