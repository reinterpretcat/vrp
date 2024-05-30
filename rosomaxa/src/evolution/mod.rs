//! Contains functionality to run evolution simulation.

use crate::prelude::*;

mod config;
pub use self::config::*;

mod simulator;
pub use self::simulator::*;

pub mod telemetry;
pub use self::telemetry::*;

pub mod objective;
pub mod strategies;

/// Defines evolution result type.
pub type EvolutionResult<S> = Result<(Vec<S>, Option<TelemetryMetrics>), GenericError>;

/// Provides the way to preprocess context before using it.
pub trait HeuristicContextProcessing {
    /// A fitness type.
    type Fitness: HeuristicFitness;
    /// A heuristic context type.
    type Context: HeuristicContext<Fitness = Self::Fitness, Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Fitness = Self::Fitness, Solution = Self::Solution>;
    /// A solution type.
    type Solution: HeuristicSolution<Fitness = Self::Fitness>;

    /// Preprocess a context in order to replace usages of a given context with a new one.
    fn pre_process(&self, context: Self::Context) -> Self::Context;
}

/// Provides the way to modify solution before returning it.
pub trait HeuristicSolutionProcessing {
    /// A fitness type.
    type Fitness: HeuristicFitness;
    /// A solution type.
    type Solution: HeuristicSolution<Fitness = Self::Fitness>;

    /// Post processes solution.
    fn post_process(&self, solution: Self::Solution) -> Self::Solution;
}
