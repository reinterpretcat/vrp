#[cfg(test)]
#[path = "../../tests/unit/termination/min_variation_test.rs"]
mod min_variation_test;

use super::*;
use crate::algorithms::math::get_cv;
use crate::utils::{CollectGroupBy, UnwrapValue};
use rand::prelude::SliceRandom;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::ControlFlow;

/// A termination criteria which calculates coefficient variation in each objective and terminates
/// when min threshold is not reached.
pub struct MinVariation<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    K: Hash + Eq + Clone,
{
    interval_type: IntervalType,
    threshold: Float,
    is_global: bool,
    key: K,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

enum IntervalType {
    Sample(usize),
    Period(u128),
}

impl<C, O, S, K> MinVariation<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    K: Hash + Eq + Clone,
{
    /// Creates a new instance of `MinVariation` with sample interval type.
    pub fn new_with_sample(sample: usize, threshold: Float, is_global: bool, key: K) -> Self {
        assert_ne!(sample, 0);
        Self::new(IntervalType::Sample(sample), threshold, is_global, key)
    }

    /// Creates a new instance of `MinVariation` with period interval type.
    pub fn new_with_period(period: usize, threshold: Float, is_global: bool, key: K) -> Self {
        assert_ne!(period, 0);
        Self::new(IntervalType::Period(period as u128 * 1000), threshold, is_global, key)
    }

    fn new(interval_type: IntervalType, threshold: Float, is_global: bool, key: K) -> Self {
        Self {
            interval_type,
            threshold,
            is_global,
            key,
            _marker: (Default::default(), Default::default(), Default::default()),
        }
    }

    fn update_and_check(&self, heuristic_ctx: &mut C, fitness: Vec<Float>) -> bool {
        match &self.interval_type {
            IntervalType::Sample(sample) => {
                let generation = heuristic_ctx.statistics().generation;

                let values = heuristic_ctx
                    .state_mut::<Vec<Vec<Float>>, _>(self.key.clone(), || vec![vec![0.; fitness.len()]; *sample]);

                values[generation % sample] = fitness;

                if generation < (*sample - 1) { false } else { self.check_threshold(values.iter()) }
            }
            IntervalType::Period(period) => {
                let elapsed_time = heuristic_ctx.statistics().time.elapsed_millis();
                let random = heuristic_ctx.environment().random.clone();
                let mut rng = random.get_rng();

                let values = heuristic_ctx
                    .state_mut::<Vec<(u128, Vec<Float>)>, _>(self.key.clone(), Vec::<(u128, Vec<Float>)>::default);

                values.push((elapsed_time, fitness));

                // NOTE try to keep collection under maintainable size
                if values.len() > 1000 {
                    let mut i = 0_usize;
                    values.shuffle(&mut rng);
                    values.retain(|_| {
                        let result = i % 10 == 0;
                        i += 1;

                        result
                    });
                    values.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
                }

                if *period > elapsed_time || values.len() < 2 {
                    false
                } else {
                    let earliest = elapsed_time - *period;
                    let position = values.iter().rev().position(|(time, _)| *time < earliest);

                    let position = match position {
                        Some(position) if position < 2 && values.len() < 3 => 0,
                        Some(position) if position < 2 && values.len() > 3 => values.len() - 2,
                        Some(position) => values.len() - position,
                        _ => 0,
                    };

                    values.drain(0..position);

                    self.check_threshold(values.iter().map(|(_, fitness)| fitness))
                }
            }
        }
    }

    fn check_threshold<'a, I>(&self, values: I) -> bool
    where
        I: Iterator<Item = &'a Vec<Float>>,
    {
        values
            .flat_map(|values| values.iter().cloned().enumerate())
            .collect_group_by()
            .into_iter()
            .try_fold(true, |_, (_, values)| {
                let cv = get_cv(values.as_slice());
                if cv > self.threshold { ControlFlow::Break(false) } else { ControlFlow::Continue(true) }
            })
            .unwrap_value()
    }
}

impl<C, O, S, K> Termination for MinVariation<C, O, S, K>
where
    C: HeuristicContext<Objective = O, Solution = S> + Stateful<Key = K>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    K: Hash + Eq + Clone + Send + Sync,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool {
        let first_individual = heuristic_ctx.ranked().next();
        if let Some(first) = first_individual {
            let fitness = first.fitness().collect::<Vec<_>>();
            let result = self.update_and_check(heuristic_ctx, fitness);

            match (self.is_global, heuristic_ctx.selection_phase()) {
                (true, _) => result,
                (false, SelectionPhase::Exploitation) => result,
                _ => false,
            }
        } else {
            false
        }
    }

    fn estimate(&self, _: &Self::Context) -> Float {
        0.
    }
}
