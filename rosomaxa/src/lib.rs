//! This crate exposes a generalized hyper heuristics and some helper functionality which can be
//! used to build a solver for optimization problems.
//!
//!
//! # Examples
//!
//! This example demonstrates the usage of example models and heuristics to minimize Rosenbrock function.
//! For the sake of minimalism, there is a pre-built solver and heuristic operator models. Check
//! example module to see how to use functionality of the crate for an arbitrary domain.
//!
//! ```
//! # use std::sync::Arc;
//! use rosomaxa::prelude::*;
//! use rosomaxa::example::*;
//! let random = Arc::new(DefaultRandom::default());
//! // an example of heuristic operator, it is domain specific
//! let noise_op = VectorHeuristicOperatorMode::JustNoise(Noise::new(1., (-0.1, 0.1), random));
//!
//! // add some configuration and run the solver
//! let (solutions, _) = Solver::default()
//!     .with_fitness_fn(create_rosenbrock_function())
//!     .with_init_solutions(vec![vec![2., 2.]])
//!     .with_operator(noise_op, "first", 1.)
//!     .with_termination(Some(5), Some(1000), None, None)
//!     .solve()
//!     .expect("cannot build and use solver");
//!
//! // expecting at least one solution with fitness close to 0
//! assert_eq!(solutions.len(), 1);
//! let (_, fitness) = solutions.first().unwrap();
//! assert!(*fitness < 0.01);
//!
//! # Ok::<(), String>(())
//! ```
//!

#![warn(missing_docs)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod algorithms;
pub mod evolution;
pub mod example;
pub mod hyper;
pub mod population;
pub mod prelude;
pub mod termination;
pub mod utils;

use crate::algorithms::nsga2::MultiObjective;
use crate::evolution::{Telemetry, TelemetryMetrics, TelemetryMode};
use crate::population::*;
use crate::utils::Environment;
use crate::utils::Timer;
use std::hash::Hash;
use std::sync::Arc;

/// Represents solution in population defined as actual solution.
pub trait HeuristicSolution: Send + Sync {
    /// Get fitness values of a given solution.
    fn get_fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a>;
    /// Creates a deep copy of the solution.
    fn deep_copy(&self) -> Self;
}

/// Represents a heuristic objective function.
pub trait HeuristicObjective: MultiObjective + Send + Sync {}
/// Specifies a dynamically dispatched type for heuristic population.
pub type DynHeuristicPopulation<O, S> = dyn HeuristicPopulation<Objective = O, Individual = S>;
/// Specifies a heuristic result type.
pub type HeuristicResult<O, S> = Result<(Box<DynHeuristicPopulation<O, S>>, Option<TelemetryMetrics>), String>;

/// Represents heuristic context.
pub trait HeuristicContext: Send + Sync {
    /// A heuristic objective function type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Returns objective function used by the population.
    fn objective(&self) -> &Self::Objective;

    /// Returns current population.
    fn population(&self) -> &DynHeuristicPopulation<Self::Objective, Self::Solution>;

    /// Returns current statistic used to track the search progress.
    fn statistics(&self) -> &HeuristicStatistics;

    /// Returns environment.
    fn environment(&self) -> &Environment;

    /// Updates population with initial solution.
    fn on_initial(&mut self, solution: Self::Solution, item_time: Timer);

    /// Updates population with a new offspring.
    fn on_generation(&mut self, offspring: Vec<Self::Solution>, termination_estimate: f64, generation_time: Timer);

    /// Returns final population and telemetry metrics
    fn on_result(self) -> HeuristicResult<Self::Objective, Self::Solution>;
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
            speed: HeuristicSpeed::Unknown,
            improvement_all_ratio: 0.,
            improvement_1000_ratio: 0.,
            termination_estimate: 0.,
        }
    }
}

/// A default heuristic context implementation which uses telemetry to track search progression parameters.
pub struct TelemetryHeuristicContext<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    objective: Arc<O>,
    population: Box<DynHeuristicPopulation<O, S>>,
    telemetry: Telemetry<O, S>,
    environment: Arc<Environment>,
}

impl<O, S> TelemetryHeuristicContext<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `TelemetryHeuristicContext`.
    pub fn new(
        objective: Arc<O>,
        population: Box<DynHeuristicPopulation<O, S>>,
        telemetry_mode: TelemetryMode,
        environment: Arc<Environment>,
    ) -> Self {
        let telemetry = Telemetry::new(telemetry_mode);
        Self { objective, population, telemetry, environment }
    }

    /// Adds solution to population.
    pub fn add_solution(&mut self, solution: S) {
        self.population.add(solution);
    }
}

impl<O, S> HeuristicContext for TelemetryHeuristicContext<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Objective = O;
    type Solution = S;

    fn objective(&self) -> &Self::Objective {
        &self.objective
    }

    fn population(&self) -> &DynHeuristicPopulation<Self::Objective, Self::Solution> {
        self.population.as_ref()
    }

    fn statistics(&self) -> &HeuristicStatistics {
        self.telemetry.get_statistics()
    }

    fn environment(&self) -> &Environment {
        self.environment.as_ref()
    }

    fn on_initial(&mut self, solution: Self::Solution, item_time: Timer) {
        self.telemetry.on_initial(&solution, item_time);
        self.population.add(solution);
    }

    fn on_generation(&mut self, offspring: Vec<Self::Solution>, termination_estimate: f64, generation_time: Timer) {
        let is_improved = self.population.add_all(offspring);
        self.telemetry.on_generation(
            self.objective.as_ref(),
            self.population.as_ref(),
            termination_estimate,
            generation_time,
            is_improved,
        );
        self.population.on_generation(self.telemetry.get_statistics());
    }

    fn on_result(self) -> Result<(Box<DynHeuristicPopulation<O, S>>, Option<TelemetryMetrics>), String> {
        let mut telemetry = self.telemetry;

        telemetry.on_result(self.objective.as_ref(), self.population.as_ref());

        Ok((self.population, telemetry.take_metrics()))
    }
}

/// Defines instant refinement speed type.
#[derive(Clone)]
pub enum HeuristicSpeed {
    /// Not yet calculated
    Unknown,

    /// Slow speed.
    Slow {
        /// Ratio.
        ratio: f64,
        /// Average refinement speed in generations per second.
        average: f64,
    },

    /// Moderate speed.
    Moderate {
        /// Average refinement speed in generations per second.
        average: f64,
    },
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

/// Gets default population selection size.
pub fn get_default_selection_size(environment: &Environment) -> usize {
    environment.parallelism.available_cpus().min(8)
}

/// Gets default population algorithm.
pub fn get_default_population<O, S>(
    objective: Arc<O>,
    environment: Arc<Environment>,
    selection_size: usize,
) -> Box<dyn HeuristicPopulation<Objective = O, Individual = S> + Send + Sync>
where
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    if selection_size == 1 {
        Box::new(Greedy::new(objective, 1, None))
    } else {
        let config = RosomaxaConfig::new_with_defaults(selection_size);
        let population =
            Rosomaxa::new(objective, environment, config).expect("cannot create rosomaxa with default configuration");

        Box::new(population)
    }
}
