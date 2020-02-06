#[cfg(test)]
#[path = "../../../tests/unit/refinement/termination/variation_coefficient_test.rs"]
mod variation_coefficient_test;

use crate::refinement::termination::Termination;
use crate::refinement::{Individuum, RefinementContext};

/// Uses coefficient of variation as termination criteria.
pub struct VariationCoefficient {
    capacity: usize,
    threshold: f64,
}

/// Keeps data needed to calculate variation coefficient between generations.
struct VariationCoefficientState {
    pub last_cost: Option<f64>,
    pub costs: Vec<f64>,
}

impl VariationCoefficientState {
    fn new(capacity: usize) -> Self {
        Self { last_cost: None, costs: vec![0.; capacity] }
    }
}

impl Termination for VariationCoefficient {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, solution: (&Individuum, bool)) -> bool {
        let (individuum, is_accepted) = solution;

        let mut state = refinement_ctx
            .state
            .entry("var_coeff".to_owned())
            .or_insert_with(|| Box::new(VariationCoefficientState::new(self.capacity)))
            .downcast_mut::<VariationCoefficientState>()
            .unwrap();

        if is_accepted {
            state.last_cost = Some(individuum.1.total());
        }

        let index = refinement_ctx.generation % self.capacity;

        state.costs[index] = individuum.1.total();

        refinement_ctx.generation >= (self.capacity - 1) && self.check_threshold(state)
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
        Self { capacity, threshold }
    }

    fn check_threshold(&self, state: &VariationCoefficientState) -> bool {
        let sum: f64 = state.costs.iter().sum();
        let mean = sum / self.capacity as f64;
        let variance = self.calculate_variance(state, mean);
        let sdev = variance.sqrt();
        let cv = sdev / mean;

        cv < self.threshold
    }

    fn calculate_variance(&self, state: &VariationCoefficientState, mean: f64) -> f64 {
        let (first, second) = state.costs.iter().fold((0., 0.), |acc, v| {
            let dev = v - mean;
            (acc.0 + dev * dev, acc.1 + dev)
        });

        (first - (second * second / self.capacity as f64)) / (self.capacity as f64 - 1.)
    }
}
