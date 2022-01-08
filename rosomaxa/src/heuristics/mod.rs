//! This module contains heuristic-related functionality.

use crate::algorithms::nsga2::MultiObjective;
use crate::heuristics::population::HeuristicPopulation;
use crate::utils::Environment;

pub mod hyper;
pub mod population;

/// Represents solution in population defined as actual solution.
pub trait HeuristicSolution: Send + Sync {
    /// Get fitness values of a given solution.
    fn get_fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a>;
    /// Creates a deep copy of the solution.
    fn deep_copy(&self) -> Self;
}

/// Represents a heuristic objective function.
pub trait HeuristicObjective: MultiObjective + Send + Sync {}

/// Represents heuristic context.
pub trait HeuristicContext: Send + Sync {
    /// A heuristic objective function type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A population type used by heuristic.
    type Population: HeuristicPopulation<Objective = Self::Objective, Individual = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Returns objective function used by the population.
    fn objective(&self) -> &<Self::Population as HeuristicPopulation>::Objective;

    /// Returns population.
    fn population(&self) -> &Self::Population;

    /// Returns current statistic used to track the search progress.
    fn statistics(&self) -> &HeuristicStatistics;

    /// Returns environment.
    fn environment(&self) -> &Environment;
}

/// Defines instant refinement speed type.
#[derive(Clone)]
pub enum HeuristicSpeed {
    /// Slow speed with ratio estimation
    Slow(f64),

    /// Moderate speed.
    Moderate,
}

/// A refinement statistics to track evolution progress.
#[derive(Clone)]
pub struct HeuristicStatistics {
    /// A number which specifies refinement generation.
    pub generation: usize,

    /// A current refinement speed.
    pub speed: HeuristicSpeed,

    /// An improvement ratio from beginning.
    pub improvement_all_ratio: f64,

    /// An improvement ratio over last 1000 iterations.
    pub improvement_1000_ratio: f64,

    /// A progress till algorithm's termination.
    pub termination_estimate: f64,
}
