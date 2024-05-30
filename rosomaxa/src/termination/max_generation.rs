#[cfg(test)]
#[path = "../../tests/unit/termination/max_generation_test.rs"]
mod max_generation_test;

use super::*;
use std::marker::PhantomData;

/// A termination criteria which is in terminated state when maximum amount of generations is exceeded.
pub struct MaxGeneration<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    limit: usize,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<F, C, O, S> MaxGeneration<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    /// Creates a new instance of `MaxGeneration`.
    pub fn new(limit: usize) -> Self {
        Self { limit, _marker: (Default::default(), Default::default(), Default::default()) }
    }
}

impl<F, C, O, S> Termination for MaxGeneration<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
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
