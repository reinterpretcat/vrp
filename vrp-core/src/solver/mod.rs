//! The *solver* module contains basic building blocks for a metaheuristic among with the default
//! implementation.
//!
//! # Metaheuristic
//!
//! A metaheuristic is a high-level algorithmic framework that provides a set of guidelines or strategies
//! to develop heuristic optimization algorithms. Examples of metaheuristics include genetic/evolutionary
//! algorithms, tabu search, simulated annealing, variable neighborhood search, (adaptive) large
//! neighborhood search, ant colony optimization, etc.
//!
//!
//! # Multi-objective decision maker
//!
//! Most VRPs, frequently used to model real cases, are set up with a single objective (e.g. minimizing
//! the cost of the solution), however the majority of the problems encountered in logistics industry,
//! are multi-objective in nature as the complexity of real-life logistics planning often cannot be
//! reduced to cost only. Such non-cost factors are:
//!
//! - **balancing work across multiple workers**
//! - **minimization or maximization of fleet usage**
//! - **minimization of unassigned jobs**
//!
//! In most of the cases, these additional factors are contradicting to the cost minimization
//! objective which, in fact, leads to nontrivial multi-objective optimization problem, where no
//! single solution exists that simultaneously optimizes each objective.
//!
//! # Evolutionary algorithm
//!
//! An evolutionary algorithm (EA) is a generic population-based metaheuristic optimization algorithm.
//! This crate provides a custom implementation of EA which can be divided into the following steps:
//!
//! - **initialization**: on this step, an initial population is created using different construction heuristics.
//! - **main loop begin**: enter an evolution loop
//!     - **selection**: an individual is selected from population. Best-fit individuals have more chances to be selected.
//!     - **mutation**: a mutation operator is applied to selected individual. Default implementation
//!       uses `ruin and recreate` principle described in next section.
//!     - **population adjustments**: new individual is added to population, then the population is
//!       sorted and shrinked to keep it under specific size limits with best-fit individuals and
//!       some intermediate.
//! - **main loop end**: exit evolution loop when one of termination criteria are met. See `termination` module for details.
//!
//! As there is no crossover operator involved and offspring is produced from one parent, this algorithm
//! can be characterized as parthenogenesis based EA. This approach eliminates design of feasible
//! crossover operator which is a challenging task in case of VRP.
//!
//! # Population
//!
//! A custom algorithm is implemented to maintain diversity and guide the search maintaining trade
//! of between exploration and exploitation of solution space. See `rosomaxa` crate for details.
//!
//!
//! # Ruin and Recreate principle
//!
//! A **ruin and recreate** principle is introduced by [`Schrimpf et al. (2000)`] and key idea here
//! is to ruin a quite large fraction of the solution and try to restore the solution as best as it
//! is possible in order to get a new solution better than the previous one. Original algorithm can
//! be described as a large neighborhood search that combines elements of simulated annealing and
//! threshold-accepting algorithms, but this crate only reuses ruin/recreate idea as a mutation
//! operator.
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//!
//!
//! The solver is not limited by R&R principle, additionally, it uses some other heuristics
//! and their combinations. They are picked based on their performance in terms of search quality and
//! latency introduced. Reinforcement technics are used here.
//!

extern crate rand;

use crate::construction::heuristics::InsertionContext;
use crate::models::common::{Footprint, FootprintSolutionState, Shadow};
use crate::models::{GoalContext, Problem, Solution};
use crate::solver::search::Recreate;
use rosomaxa::evolution::*;
use rosomaxa::prelude::*;
use rosomaxa::utils::Timer;
use rosomaxa::{TelemetryHeuristicContext, get_default_population};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

pub mod processing;
pub mod search;

mod heuristic;
pub use self::heuristic::*;

/// A type which encapsulates information needed to perform a solution refinement process.
pub struct RefinementContext {
    /// Original problem definition.
    pub problem: Arc<Problem>,
    /// An environmental context.
    pub environment: Arc<Environment>,
    /// A collection of data associated with a refinement process.
    pub state: HashMap<String, Box<dyn Any + Sync + Send>>,
    /// Keeps track of the initial footprint.
    initial_footprint: Footprint,
    /// Provides some basic implementation of context functionality.
    inner_context: TelemetryHeuristicContext<GoalContext, InsertionContext>,
}

/// Defines instant refinement speed type.
#[derive(Clone)]
pub enum RefinementSpeed {
    /// Slow speed with ratio estimation
    Slow(Float),

    /// Moderate speed.
    Moderate,
}

impl RefinementContext {
    /// Creates a new instance of `RefinementContext`.
    pub fn new(
        problem: Arc<Problem>,
        population: TargetPopulation,
        telemetry_mode: TelemetryMode,
        environment: Arc<Environment>,
    ) -> Self {
        let initial_footprint = Footprint::new(&problem);
        let inner_context =
            TelemetryHeuristicContext::new(problem.goal.clone(), population, telemetry_mode, environment.clone());
        Self { problem, environment, inner_context, state: Default::default(), initial_footprint }
    }

    /// Adds solution to population.
    pub fn add_solution(&mut self, solution: InsertionContext) {
        self.inner_context.add_solution(solution);
    }
}

impl HeuristicContext for RefinementContext {
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn objective(&self) -> &Self::Objective {
        self.inner_context.objective()
    }

    fn selected(&self) -> Box<dyn Iterator<Item = &'_ Self::Solution> + '_> {
        self.inner_context.selected()
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Solution> + '_> {
        self.inner_context.ranked()
    }

    fn statistics(&self) -> &HeuristicStatistics {
        self.inner_context.statistics()
    }

    fn selection_phase(&self) -> SelectionPhase {
        self.inner_context.selection_phase()
    }

    fn environment(&self) -> &Environment {
        self.inner_context.environment()
    }

    fn on_initial(&mut self, mut solution: Self::Solution, item_time: Timer) {
        self.initial_footprint.add(&Shadow::from(&solution));
        solution.solution.state.set_footprint(self.initial_footprint.clone());

        self.inner_context.on_initial(solution, item_time)
    }

    fn on_generation(&mut self, offspring: Vec<Self::Solution>, termination_estimate: Float, generation_time: Timer) {
        self.inner_context.on_generation(offspring, termination_estimate, generation_time)
    }

    fn on_result(self) -> HeuristicResult<Self::Objective, Self::Solution> {
        self.inner_context.on_result()
    }
}

impl Stateful for RefinementContext {
    type Key = String;

    fn set_state<T: 'static + Send + Sync>(&mut self, key: Self::Key, state: T) {
        self.state.insert(key, Box::new(state));
    }

    fn get_state<T: 'static + Send + Sync>(&self, key: &Self::Key) -> Option<&T> {
        self.state.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    fn state_mut<T: 'static + Send + Sync, F: Fn() -> T>(&mut self, key: Self::Key, inserter: F) -> &mut T {
        // NOTE may panic if casting fails
        self.state.entry(key).or_insert_with(|| Box::new(inserter())).downcast_mut::<T>().unwrap()
    }
}

/// Wraps recreate method as `InitialOperator`
pub struct RecreateInitialOperator {
    recreate: Arc<dyn Recreate>,
}

impl RecreateInitialOperator {
    /// Creates a new instance of `RecreateInitialOperator`.
    pub fn new(recreate: Arc<dyn Recreate>) -> Self {
        Self { recreate }
    }
}

impl InitialOperator for RecreateInitialOperator {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn create(&self, heuristic_ctx: &Self::Context) -> Self::Solution {
        let mut insertion_ctx = InsertionContext::new(heuristic_ctx.problem.clone(), heuristic_ctx.environment.clone());
        insertion_ctx.solution.state.set_footprint(heuristic_ctx.initial_footprint.clone());

        self.recreate.run(heuristic_ctx, insertion_ctx)
    }
}

/// Solves a Vehicle Routing Problem and returns a _(solution, its cost)_ pair in case of success
/// or error description, if solution cannot be found.
pub struct Solver {
    problem: Arc<Problem>,
    config: EvolutionConfig<RefinementContext, GoalContext, InsertionContext>,
}

impl Solver {
    /// Tries to create an instance of `Solver` from provided config.
    pub fn new(
        problem: Arc<Problem>,
        config: EvolutionConfig<RefinementContext, GoalContext, InsertionContext>,
    ) -> Self {
        Self { problem, config }
    }

    /// Solves a Vehicle Routing Problem and returns a feasible solution in case of success
    /// or error description if solution cannot be found.
    pub fn solve(self) -> GenericResult<Solution> {
        (self.config.context.environment.logger)(&format!(
            "total jobs: {}, actors: {}",
            self.problem.jobs.size(),
            self.problem.fleet.actors.len()
        ));

        let (mut solutions, metrics) = EvolutionSimulator::new(self.config)?.run()?;

        // NOTE select the first best individual from population
        let insertion_ctx = if solutions.is_empty() { None } else { solutions.drain(0..1).next() }
            .ok_or_else(|| "cannot find any solution".to_string())?;

        let solution = (insertion_ctx, metrics).into();

        Ok(solution)
    }
}
