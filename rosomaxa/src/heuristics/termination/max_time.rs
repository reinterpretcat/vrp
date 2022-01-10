use super::*;
use crate::utils::Timer;
use std::marker::PhantomData;

/// A termination criteria which is in terminated state when max time elapsed.
pub struct MaxTime<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    start: Timer,
    limit_in_secs: f64,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<C, O, S> MaxTime<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
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

impl<C, O, S> Termination for MaxTime<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
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
