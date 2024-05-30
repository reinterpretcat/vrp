use super::*;
use crate::utils::Timer;
use std::marker::PhantomData;

/// A termination criteria which is in terminated state when max time elapsed.
pub struct MaxTime<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    start: Timer,
    limit_in_secs: f64,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<F, C, O, S> MaxTime<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    /// Creates a new instance of `MaxTime`.
    pub fn new(limit_in_secs: f64) -> Self {
        Self {
            start: Timer::start(),
            limit_in_secs,
            _marker: (Default::default(), Default::default(), Default::default()),
        }
    }
}

impl<F, C, O, S> Termination for MaxTime<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, _: &mut Self::Context) -> bool {
        self.start.elapsed_secs_as_f64() > self.limit_in_secs
    }

    fn estimate(&self, _: &Self::Context) -> f64 {
        (self.start.elapsed_secs_as_f64() / self.limit_in_secs).min(1.)
    }
}
