//! Contains functionality to run evolution simulation.

use crate::prelude::*;

mod config;
pub use self::config::*;

mod simulator;
pub use self::simulator::*;

pub mod telemetry;
pub use self::telemetry::*;

pub mod strategies;

/// Defines evolution result type.
pub type EvolutionResult<S> = Result<(Vec<S>, Option<TelemetryMetrics>), String>;

/// Provides the way to preprocess context before using it.
pub trait HeuristicContextProcessing {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A solution type.
    type Solution: HeuristicSolution;

    /// Preprocess a context in order to replace usages of a given context with a new one.
    fn pre_process(&self, context: Self::Context) -> Self::Context;
}

/// Provides the way to modify solution before returning it.
pub trait HeuristicSolutionProcessing {
    /// A solution type.
    type Solution: HeuristicSolution;

    /// Post processes solution.
    fn post_process(&self, solution: Self::Solution) -> Self::Solution;
}
