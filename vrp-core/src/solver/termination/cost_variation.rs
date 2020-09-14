#[cfg(test)]
#[path = "../../../tests/unit/solver/termination/cost_variation_test.rs"]
mod cost_variation_test;

use crate::algorithms::nsga2::Objective;
use crate::algorithms::statistics::get_cv;
use crate::models::common::Cost;
use crate::solver::termination::Termination;
use crate::solver::RefinementContext;

/// A termination criteria which is in terminated state based on cost variation during the refinement
/// process.
pub struct CostVariation {
    sample: usize,
    threshold: f64,
    key: String,
}

impl CostVariation {
    /// Creates a new instance of `CostVariation`.
    pub fn new(sample: usize, threshold: f64) -> Self {
        Self { sample, threshold, key: "coeff_var".to_string() }
    }

    fn update_and_check(&self, refinement_ctx: &mut RefinementContext, cost: Cost) -> bool {
        let costs = refinement_ctx
            .state
            .entry(self.key.clone())
            .or_insert_with(|| Box::new(vec![0.; self.sample]))
            .downcast_mut::<Vec<f64>>()
            .unwrap();

        costs[refinement_ctx.statistics.generation % self.sample] = cost;

        refinement_ctx.statistics.generation >= (self.sample - 1) && self.check_threshold(costs)
    }

    fn check_threshold(&self, costs: &[f64]) -> bool {
        get_cv(costs) < self.threshold
    }
}

impl Termination for CostVariation {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext) -> bool {
        let first_individual = refinement_ctx.population.ranked().next();
        if let Some((first, _)) = first_individual {
            let cost = refinement_ctx.problem.objective.fitness(first);
            self.update_and_check(refinement_ctx, cost)
        } else {
            false
        }
    }
}
