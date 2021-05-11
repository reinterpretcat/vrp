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
    interval_type: IntervalType,
    threshold: f64,
    is_global: bool,
    key: String,
}

enum IntervalType {
    Sample(usize),
    Period(u64),
}

impl MinVariation {
    /// Creates a new instance of `MinVariation` with sample interval type.
    pub fn new_with_sample(sample: usize, threshold: f64, is_global: bool) -> Self {
        assert_ne!(sample, 0);
        Self::new(IntervalType::Sample(sample), threshold, is_global)
    }

    /// Creates a new instance of `MinVariation` with period interval type.
    pub fn new_with_period(period: usize, threshold: f64, is_global: bool) -> Self {
        assert_ne!(period, 0);
        Self::new(IntervalType::Period(period as u64), threshold, is_global)
    }

    fn new(interval_type: IntervalType, threshold: f64, is_global: bool) -> Self {
        Self { interval_type, threshold, is_global, key: "max_var".to_string() }
    }

    fn update_and_check(&self, refinement_ctx: &mut RefinementContext, fitness: Vec<f64>) -> bool {
        match &self.interval_type {
            IntervalType::Sample(sample) => {
                let values = refinement_ctx
                    .state
                    .entry(self.key.clone())
                    .or_insert_with(|| Box::new(vec![vec![0.; fitness.len()]; *sample]))
                    .downcast_mut::<Vec<Vec<f64>>>()
                    .unwrap();

                values[refinement_ctx.statistics.generation % sample] = fitness;

                if refinement_ctx.statistics.generation < (*sample - 1) {
                    false
                } else {
                    self.check_threshold(values.iter())
                }
            }
            IntervalType::Period(period) => {
                let values = refinement_ctx
                    .state
                    .entry(self.key.clone())
                    .or_insert_with(|| Box::new(Vec::<(u64, Vec<f64>)>::default()))
                    .downcast_mut::<Vec<(u64, Vec<f64>)>>()
                    .unwrap();

                let current = refinement_ctx.statistics.time.elapsed_secs();
                values.push((current, fitness));

                if *period > current {
                    false
                } else {
                    let earliest = current - *period;
                    let position = values.iter().rev().position(|(time, _)| *time < earliest);
                    if let Some(position) = position {
                        values.drain(0..position);
                    }

                    self.check_threshold(values.iter().map(|(_, fitness)| fitness))
                }
            }
        }
    }

    fn check_threshold<'a, I>(&self, values: I) -> bool
    where
        I: Iterator<Item = &'a Vec<f64>>,
    {
        unwrap_from_result(
            values.flat_map(|values| values.iter().cloned().enumerate()).collect_group_by().into_iter().try_fold(
                true,
                |_, (_, values)| {
                    let cv = get_cv(values.as_slice());
                    if cv > self.threshold {
                        Err(false)
                    } else {
                        Ok(true)
                    }
                },
            ),
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
