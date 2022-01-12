//! This module contains heuristic-related functionality.

use std::hash::Hash;

use crate::algorithms::nsga2::MultiObjective;
use crate::heuristics::population::HeuristicPopulation;
use crate::utils::Environment;
use crate::utils::Timer;

pub mod evolution;
pub mod hyper;
pub mod population;
pub mod termination;

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
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Returns objective function used by the population.
    fn objective(&self) -> &Self::Objective;

    /// Returns population.
    fn population(&self) -> &(dyn HeuristicPopulation<Objective = Self::Objective, Individual = Self::Solution>);

    /// Returns current statistic used to track the search progress.
    fn statistics(&self) -> &HeuristicStatistics;

    /// Returns environment.
    fn environment(&self) -> &Environment;
}

/// A refinement statistics to track evolution progress.
#[derive(Clone)]
pub struct HeuristicStatistics {
    /// A number which specifies refinement generation.
    pub generation: usize,

    /// Elapsed seconds since algorithm start.
    pub time: Timer,

    /// A current refinement speed.
    pub speed: HeuristicSpeed,

    /// An improvement ratio from beginning.
    pub improvement_all_ratio: f64,

    /// An improvement ratio over last 1000 iterations.
    pub improvement_1000_ratio: f64,

    /// A progress till algorithm's termination.
    pub termination_estimate: f64,
}

impl Default for HeuristicStatistics {
    fn default() -> Self {
        Self {
            generation: 0,
            time: Timer::start(),
            speed: HeuristicSpeed::Moderate,
            improvement_all_ratio: 0.,
            improvement_1000_ratio: 0.,
            termination_estimate: 0.,
        }
    }
}

/// Defines instant refinement speed type.
#[derive(Clone)]
pub enum HeuristicSpeed {
    /// Slow speed with ratio estimation
    Slow(f64),

    /// Moderate speed.
    Moderate,
}

/// A trait which specifies object with state behavior.
pub trait Stateful {
    /// A key type.
    type Key: Hash + Eq;

    /// Saves state using given key.
    fn set_state<T: 'static + Send + Sync>(&mut self, key: Self::Key, state: T);

    /// Tries to get state using given key.
    fn get_state<T: 'static + Send + Sync>(&self, key: &Self::Key) -> Option<&T>;

    /// Gets state as mutable, inserts if not exists.
    fn state_mut<T: 'static + Send + Sync, F: Fn() -> T>(&mut self, key: Self::Key, inserter: F) -> &mut T;
}
