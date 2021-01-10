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
use crate::algorithms::nsga2::Objective;
use crate::construction::Quota;
use crate::models::common::Cost;
use crate::models::{Problem, Solution};
use crate::solver::population::Population;
use hashbrown::HashMap;
use std::any::Any;
use std::sync::Arc;

pub mod mutation;
pub mod objectives;
pub mod population;
pub mod termination;

mod builder;
pub use self::builder::Builder;

mod evolution;
use self::evolution::{EvolutionConfig, EvolutionSimulator};

mod telemetry;
pub use self::telemetry::{Metrics, Telemetry, TelemetryMode};
use crate::utils::Environment;

/// A key to store solution order information.
const SOLUTION_ORDER_KEY: i32 = 1;

/// Keys for balancing objectives.
const BALANCE_MAX_LOAD_KEY: i32 = 20;
const BALANCE_ACTIVITY_KEY: i32 = 21;
const BALANCE_DISTANCE_KEY: i32 = 22;
const BALANCE_DURATION_KEY: i32 = 23;

/// A type which encapsulates information needed to perform solution refinement process.
pub struct RefinementContext {
    /// Original problem definition.
    pub problem: Arc<Problem>,

    /// A population which tracks best discovered solutions.
    pub population: Box<dyn Population + Sync + Send>,

    /// A collection of data associated with refinement process.
    pub state: HashMap<String, Box<dyn Any + Sync + Send>>,

    /// A quota for refinement process.
    pub quota: Option<Arc<dyn Quota + Send + Sync>>,

    /// An environmental context.
    pub environment: Arc<Environment>,

    /// A refinement statistics.
    pub statistics: Statistics,
}

/// A refinement statistics to track evolution progress.
pub struct Statistics {
    /// A number which specifies refinement generation.
    pub generation: usize,

    /// An improvement ratio from beginning.
    pub improvement_all_ratio: f64,

    /// An improvement ratio for last 1000 iterations.
    pub improvement_1000_ratio: f64,

    /// A progress till algorithm's termination.
    pub termination_estimate: f64,
}

impl RefinementContext {
    /// Creates a new instance of `RefinementContext`.
    pub fn new(
        problem: Arc<Problem>,
        population: Box<dyn Population + Sync + Send>,
        environment: Arc<Environment>,
        quota: Option<Arc<dyn Quota + Send + Sync>>,
    ) -> Self {
        Self { problem, population, state: Default::default(), quota, environment, statistics: Statistics::default() }
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Self { generation: 0, improvement_all_ratio: 0., improvement_1000_ratio: 0., termination_estimate: 0. }
    }
}

/// A Vehicle Routing Problem Solver based on evolutionary algorithm.
pub struct Solver {
    /// A VRP problem definition.
    pub problem: Arc<Problem>,
    /// An evolution configuration.
    pub config: EvolutionConfig,
}

impl Solver {
    /// Solves a Vehicle Routing Problem and returns a _(solution, its cost)_ pair in case of success
    /// or error description, if solution cannot be found.
    ///
    /// # Examples
    ///
    /// The most simple way to run solver is to use [`Builder`](./struct.Builder.html)
    /// which has preconfigured settings:
    ///
    /// ```
    /// # use vrp_core::models::examples::create_example_problem;
    /// # use std::sync::Arc;
    /// use vrp_core::solver::Builder;
    /// use vrp_core::models::Problem;
    /// use vrp_core::utils::Environment;
    ///
    /// // create your VRP problem
    /// let problem: Arc<Problem> = create_example_problem();
    /// let environment = Arc::new(Environment::default());
    /// // build solver using builder with default settings
    /// let solver = Builder::new(problem, environment).build()?;
    /// // run solver and get the best known solution within its cost.
    /// let (solution, cost, _) = solver.solve()?;
    ///
    /// assert_eq!(cost, 42.);
    /// assert_eq!(solution.routes.len(), 1);
    /// assert_eq!(solution.unassigned.len(), 0);
    /// # Ok::<(), String>(())
    /// ```
    pub fn solve(self) -> Result<(Solution, Cost, Option<Metrics>), String> {
        let (population, metrics) = EvolutionSimulator::new(self.config)?.run()?;

        // NOTE select the first best individual from population
        let (insertion_ctx, _) = population.ranked().next().ok_or_else(|| "cannot find any solution".to_string())?;
        let solution = insertion_ctx.solution.to_solution(self.problem.extras.clone());
        let cost = self.problem.objective.fitness(insertion_ctx);

        Ok((solution, cost, metrics))
    }
}
