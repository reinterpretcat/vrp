//! Specifies evolution strategies.

use super::*;

mod iterative;
pub use self::iterative::Iterative;

/// An evolution algorithm strategy.
pub trait EvolutionStrategy {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A solution type.
    type Solution: HeuristicSolution;

    /// Runs evolution and returns a population with solution(-s).
    fn run(
        &self,
        heuristic_ctx: Self::Context,
        heuristic: Box<
            dyn HyperHeuristic<Context = Self::Context, Objective = Self::Objective, Solution = Self::Solution>,
        >,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution>;
}
