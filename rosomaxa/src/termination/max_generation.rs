#[cfg(test)]
#[path = "../../tests/unit/termination/max_generation_test.rs"]
mod max_generation_test;

use super::*;
use std::marker::PhantomData;

/// A termination criteria which is in terminated state when maximum amount of generations is exceeded.
pub struct MaxGeneration<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    limit: usize,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<C, O, S> MaxGeneration<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `MaxGeneration`.
    pub fn new(limit: usize) -> Self {
        Self { limit, _marker: (Default::default(), Default::default(), Default::default()) }
    }
}

impl<C, O, S> Termination for MaxGeneration<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool {
        heuristic_ctx.statistics().generation >= self.limit
    }

    fn estimate(&self, heuristic_ctx: &Self::Context) -> f64 {
        (heuristic_ctx.statistics().generation as f64 / self.limit as f64).min(1.)
    }
}
