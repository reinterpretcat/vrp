#[cfg(test)]
#[path = "../../tests/unit/termination/target_proximity_test.rs"]
mod target_proximity_test;

use super::*;
use crate::algorithms::math::relative_distance;
use std::marker::PhantomData;

/// Provides way to set stop algorithm when some close solution is found.
pub struct TargetProximity<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    target_fitness: F,
    distance_threshold: f64,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<F, C, O, S> TargetProximity<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    /// Creates a new instance of `TargetProximity`.
    pub fn new(target_fitness: F, distance_threshold: f64) -> Self {
        Self {
            target_fitness,
            distance_threshold,
            _marker: (Default::default(), Default::default(), Default::default()),
        }
    }
}

impl<F, C, O, S> Termination for TargetProximity<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool {
        // NOTE ignore pareto front, use the first solution only for comparison
        heuristic_ctx.ranked().next().map_or(false, |solution| {
            let distance = relative_distance(self.target_fitness.iter(), solution.fitness().iter());
            distance < self.distance_threshold
        })
    }

    fn estimate(&self, _: &Self::Context) -> f64 {
        0.
    }
}
