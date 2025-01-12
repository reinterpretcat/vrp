#[cfg(test)]
#[path = "../../tests/unit/termination/target_proximity_test.rs"]
mod target_proximity_test;

use super::*;
use crate::algorithms::math::relative_distance;
use std::marker::PhantomData;

/// Provides a way to set stop algorithm when some close solution is found.
pub struct TargetProximity<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    target_fitness: Vec<Float>,
    distance_threshold: Float,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<C, O, S> TargetProximity<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `TargetProximity`.
    pub fn new(target_fitness: Vec<Float>, distance_threshold: Float) -> Self {
        Self {
            target_fitness,
            distance_threshold,
            _marker: (Default::default(), Default::default(), Default::default()),
        }
    }
}

impl<C, O, S> Termination for TargetProximity<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool {
        // use the first solution only for comparison
        heuristic_ctx.ranked().next().is_some_and(|solution| {
            let distance = relative_distance(self.target_fitness.iter().cloned(), solution.fitness());
            distance < self.distance_threshold
        })
    }

    fn estimate(&self, _: &Self::Context) -> Float {
        0.
    }
}
