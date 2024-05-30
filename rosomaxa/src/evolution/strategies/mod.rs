//! Specifies evolution strategies.

use super::*;

mod iterative;
pub use self::iterative::Iterative;

/// An evolution algorithm strategy.
pub trait EvolutionStrategy {
    /// A heuristic fitness type.
    type Fitness: HeuristicFitness;
    /// A heuristic context type.
    type Context: HeuristicContext<Fitness = Self::Fitness, Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Fitness = Self::Fitness, Solution = Self::Solution>;
    /// A solution type.
    type Solution: HeuristicSolution;

    /// Runs evolution and returns a population with solution(-s).
    fn run(
        &mut self,
        heuristic_ctx: Self::Context,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution>;
}
