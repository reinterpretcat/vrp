//! The termination module contains logic which defines termination criteria for metaheuristic,
//! e.g. when to stop evolution in evolutionary algorithms.

use crate::prelude::*;

/// A trait which specifies criteria when metaheuristic should stop searching for improved solution.
pub trait Termination {
    /// A heuristic objective function type.
    type Context: HeuristicContext<Objective = Self::Objective>;

    /// A heuristic objective type.
    type Objective: HeuristicObjective;

    /// Returns true if termination condition is met.
    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool;

    /// Returns a relative estimation till termination. Value is in the `[0, 1]` range.
    fn estimate(&self, heuristic_ctx: &Self::Context) -> f64;
}

mod min_variation;
pub use self::min_variation::MinVariation;

mod max_generation;
pub use self::max_generation::MaxGeneration;

mod max_time;
pub use self::max_time::MaxTime;

/// A trait which encapsulates multiple termination criteria.
pub struct CompositeTermination<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    terminations: Vec<Box<dyn Termination<Context = C, Objective = O> + Send + Sync>>,
}

impl<C, O, S> CompositeTermination<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `CompositeTermination`.
    pub fn new(terminations: Vec<Box<dyn Termination<Context = C, Objective = O> + Send + Sync>>) -> Self {
        Self { terminations }
    }
}

impl<C, O, S> Termination for CompositeTermination<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;

    fn is_termination(&self, heuristic_ctx: &mut Self::Context) -> bool {
        self.terminations.iter().any(|t| t.is_termination(heuristic_ctx))
    }

    fn estimate(&self, heuristic_ctx: &Self::Context) -> f64 {
        self.terminations.iter().map(|t| t.estimate(heuristic_ctx)).max_by(|a, b| compare_floats(*a, *b)).unwrap_or(0.)
    }
}
