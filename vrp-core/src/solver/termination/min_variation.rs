#[cfg(test)]
#[path = "../../../tests/unit/solver/termination/min_variation_test.rs"]
mod min_variation_test;

use crate::algorithms::nsga2::MultiObjective;
use crate::algorithms::statistics::get_cv;
use crate::solver::population::SelectionPhase;
use crate::solver::termination::Termination;
use crate::solver::RefinementContext;
use crate::utils::{unwrap_from_result, CollectGroupBy};

/// A termination criteria which calculates coefficient variation in each objective and terminates
/// when min threshold is not reached.
pub struct MinVariation {
    sample: usize,
    threshold: f64,
    is_global: bool,
    key: String,
}

impl MinVariation {
    /// Creates a new instance of `MinVariation`.
    pub fn new(sample: usize, threshold: f64, is_global: bool) -> Self {
        Self { sample, threshold, is_global, key: "max_var".to_string() }
    }

    fn update_and_check(&self, refinement_ctx: &mut RefinementContext, fitness: Vec<f64>) -> bool {
        let values = refinement_ctx
            .state
            .entry(self.key.clone())
            .or_insert_with(|| Box::new(vec![vec![0.; fitness.len()]; self.sample]))
            .downcast_mut::<Vec<Vec<f64>>>()
            .unwrap();

        values[refinement_ctx.statistics.generation % self.sample] = fitness;

        refinement_ctx.statistics.generation >= (self.sample - 1) && self.check_threshold(values)
    }

    fn check_threshold(&self, values: &[Vec<f64>]) -> bool {
        unwrap_from_result(
            values
                .iter()
                .flat_map(|values| values.iter().cloned().enumerate())
                .collect_group_by()
                .into_iter()
                .try_fold(true, |_, (_, values)| {
                    let cv = get_cv(values.as_slice());
                    if cv > self.threshold {
                        Err(false)
                    } else {
                        Ok(true)
                    }
                }),
        )
    }
}

impl Termination for MinVariation {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext) -> bool {
        let first_individual = refinement_ctx.population.ranked().next();
        if let Some((first, _)) = first_individual {
            let objective = refinement_ctx.problem.objective.as_ref();
            let fitness = objective.objectives().map(|o| o.fitness(first)).collect::<Vec<_>>();
            let result = self.update_and_check(refinement_ctx, fitness);

            match (self.is_global, refinement_ctx.population.selection_phase()) {
                (true, _) => result,
                (false, SelectionPhase::Exploitation) => result,
                _ => false,
            }
        } else {
            false
        }
    }

    fn estimate(&self, _: &RefinementContext) -> f64 {
        0.
    }
}
