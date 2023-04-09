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
//! The default implementation can be roughly described as "*Multi-objective Parthenogenesis based
//! Evolutionary Algorithm with Ruin and Recreate Mutation Operator*".
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
//! That's why the concept of dominance is introduced: a solution is said to dominate another
//! solution if its quality is at least as good on every objective and better on at least one.
//! The set of all non-dominated solutions of an optimization problem is called the Pareto set and
//! the projection of this set onto the objective function space is called the Pareto front.
//!
//! The aim of multi-objective metaheuristics is to approximate the Pareto front as closely as
//! possible (Zitzler et al., 2004) and therefore generate a set of mutually non-dominated solutions
//! called the Pareto set approximation.
//!
//! This library utilizes `NSGA-II` algorithm to apply Pareto-based ranking over population in order
//! to find Pareto set approximation. However, that Pareto optimality of the solutions cannot be
//! guaranteed: it is only known that none of the generated solutions dominates the others.
//! In the end, the top ranked individual is returned as best known solution.
//!
//! This crate contains NSGA-II buildings blocks which can be found in [`nsga2`] module.
//!
//! [`nsga2`]: ../algorithms/nsga2/index.html
//!
//! # Evolutionary algorithm
//!
//! An evolutionary algorithm (EA) is a generic population-based metaheuristic optimization algorithm.
//! This crate provides a custom implementation of EA which can be divided into the following steps:
//!
//! - **initialization**: on this step, an initial population is created using different construction
//!    heuristics.
//! - **main loop begin**: enter an evolution loop
//!     - **selection**: an individual is selected from population. Best-fit individuals have more
//!        chances to be selected.
//!     - **mutation**: a mutation operator is applied to selected individual. Default implementation
//!       uses `ruin and recreate` principle described in next section.
//!     - **population adjustments**: new individual is added to population, then the population is
//!       sorted and shrinked to keep it under specific size limits with best-fit individuals and
//!       some intermediate.
//! - **main loop end**: exit evolution loop when one of termination criteria are met. See [`termination`]
//!       module for details.
//!
//! As there is no crossover operator involved and offspring is produced from one parent, this algorithm
//! can be characterized as parthenogenesis based EA. This approach eliminates design of feasible
//! crossover operator which is a challenging task in case of VRP.
//!
//!  [`termination`]: termination/index.html
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
//! Implementation blocks can be found in [`mutation`] module.
//!
//! [`Schrimpf et al. (2000)`]: https://www.sciencedirect.com/science/article/pii/S0021999199964136
//! [`mutation`]: mutation/index.html
//!
//! # Solver usage
//!
//! Check [`Builder`] and [`Solver`] documentation to see how to run VRP solver.
//!
//! [`Builder`]: ./struct.Builder.html
//! [`Solver`]: ./struct.Solver.html
//!

extern crate rand;

use crate::construction::heuristics::InsertionContext;
use crate::models::common::Cost;
use crate::models::{GoalContext, Problem, Solution};
use crate::solver::search::Recreate;
use hashbrown::HashMap;
use rosomaxa::evolution::*;
use rosomaxa::prelude::*;
use rosomaxa::{get_default_population, DynHeuristicPopulation, TelemetryHeuristicContext};
use std::any::Any;
use std::sync::Arc;

pub use self::heuristic::*;
use rosomaxa::population::Rosomaxa;
use rosomaxa::utils::Timer;

pub mod processing;
pub mod search;

mod heuristic;

/// A key to store a filter for heuristic methods applied by dynamic hyper-heuristic.
pub const HEURISTIC_FILTER_KEY: &str = "heuristic_filter";

/// A key to store solution order information.
const SOLUTION_ORDER_KEY: i32 = 1;
/// A key to store solution weights information.
const SOLUTION_WEIGHTS_KEY: i32 = 2;

/// A type which encapsulates information needed to perform solution refinement process.
pub struct RefinementContext {
    /// Original problem definition.
    pub problem: Arc<Problem>,
    /// An environmental context.
    pub environment: Arc<Environment>,
    /// A collection of data associated with refinement process.
    pub state: HashMap<String, Box<dyn Any + Sync + Send>>,
    /// Provides some basic implementation of context functionality.
    inner_context: TelemetryHeuristicContext<GoalContext, InsertionContext>,
}

/// Defines instant refinement speed type.
#[derive(Clone)]
pub enum RefinementSpeed {
    /// Slow speed with ratio estimation
    Slow(f64),

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
        let inner_context =
            TelemetryHeuristicContext::new(problem.goal.clone(), population, telemetry_mode, environment.clone());
        Self { problem, environment, inner_context, state: Default::default() }
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

    fn population(&self) -> &DynHeuristicPopulation<Self::Objective, Self::Solution> {
        self.inner_context.population()
    }

    fn statistics(&self) -> &HeuristicStatistics {
        self.inner_context.statistics()
    }

    fn environment(&self) -> &Environment {
        self.inner_context.environment()
    }

    fn on_initial(&mut self, solution: Self::Solution, item_time: Timer) {
        self.inner_context.on_initial(solution, item_time)
    }

    fn on_generation(&mut self, offspring: Vec<Self::Solution>, termination_estimate: f64, generation_time: Timer) {
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
    recreate: Arc<dyn Recreate + Send + Sync>,
}

impl RecreateInitialOperator {
    /// Creates a new instance of `RecreateInitialOperator`.
    pub fn new(recreate: Arc<dyn Recreate + Send + Sync>) -> Self {
        Self { recreate }
    }
}

impl InitialOperator for RecreateInitialOperator {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn create(&self, heuristic_ctx: &Self::Context) -> Self::Solution {
        let insertion_ctx = InsertionContext::new(heuristic_ctx.problem.clone(), heuristic_ctx.environment.clone());
        self.recreate.run(heuristic_ctx, insertion_ctx)
    }
}

/// Solves a Vehicle Routing Problem and returns a _(solution, its cost)_ pair in case of success
/// or error description, if solution cannot be found.
///
/// A newly created builder instance is pre-configured with some reasonable defaults for mid-size
/// problems (~200), so there is no need to call any of its methods.
///
///
/// # Examples
///
/// This example shows how to construct default configuration for the solver, override some of default
/// metaheuristic parameters using fluent interface methods, and run the solver:
///
/// ```
/// # use vrp_core::models::examples::create_example_problem;
/// # use vrp_core::solver::get_default_telemetry_mode;
/// # use std::sync::Arc;
/// use vrp_core::prelude::*;
///
/// // create your VRP problem
/// let problem: Arc<Problem> = create_example_problem();
/// let environment = Arc::new(Environment::new_with_time_quota(Some(60)));
/// let telemetry_mode = get_default_telemetry_mode(environment.logger.clone());
/// // build solver config using pre-build builder with defaults and then override some parameters
/// let config = create_default_config_builder(problem.clone(), environment, telemetry_mode)
///     .with_max_time(Some(60))
///     .with_max_generations(Some(100))
///     .build()?;
///
/// // run solver and get the best known solution within its cost. Telemetry metrics are ignored.
/// let (solution, cost, _) = Solver::new(problem, config).solve()?;
///
/// assert_eq!(cost, 42.);
/// assert_eq!(solution.routes.len(), 1);
/// assert_eq!(solution.unassigned.len(), 0);
/// # Ok::<(), String>(())
/// ```
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

    /// Solves a Vehicle Routing Problem and returns a _(solution, its cost)_ pair in case of success
    /// or error description, if solution cannot be found.
    pub fn solve(self) -> Result<(Solution, Cost, Option<TelemetryMetrics>), String> {
        (self.config.context.environment.logger)(&format!(
            "total jobs: {}, actors: {}",
            self.problem.jobs.size(),
            self.problem.fleet.actors.len()
        ));

        let (mut solutions, metrics) = EvolutionSimulator::new(self.config)?.run()?;

        // NOTE select the first best individual from population
        let insertion_ctx = if solutions.is_empty() { None } else { solutions.drain(0..1).next() }
            .ok_or_else(|| "cannot find any solution".to_string())?;

        let cost = insertion_ctx.solution.get_total_cost();
        let solution = insertion_ctx.solution.to_solution();

        Ok((solution, cost, metrics))
    }
}
