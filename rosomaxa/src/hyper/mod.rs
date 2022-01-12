//! This module contains a hyper-heuristic logic.

mod dynamic_selective;
pub use self::dynamic_selective::*;

mod static_selective;
pub use self::static_selective::*;

use crate::prelude::*;
use std::marker::PhantomData;
use std::ops::Deref;

/// A heuristic operator which is responsible to change passed solution.
pub trait HeuristicOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs search for a new (better) solution using given one.
    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution;
}

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a new search in the solution space using selected solutions.
    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;
}

/// A selective heuristic which uses dynamic or static selective heuristic depending on search performance.
pub struct MultiSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    actual: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
    slow: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
    is_slow_search: bool,
}

impl<C, O, S> HyperHeuristic for MultiSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        self.is_slow_search = match (self.is_slow_search, &heuristic_ctx.statistics().speed) {
            (false, HeuristicSpeed::Slow(ratio)) => {
                heuristic_ctx.environment().logger.deref()(&format!(
                    "slow refinement speed ({}), switch to slower hyper-heuristic",
                    *ratio
                ));

                std::mem::swap(&mut self.actual, &mut self.slow);

                true
            }
            (true, HeuristicSpeed::Slow(_)) => true,
            _ => false,
        };

        self.actual.search(heuristic_ctx, solutions)
    }
}

impl<C, O, S> MultiSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `MultiSelective` heuristic.
    pub fn new(
        init_heuristic: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
        slow_fallback_heuristic: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
    ) -> Self {
        Self { actual: init_heuristic, slow: slow_fallback_heuristic, is_slow_search: false }
    }
}
