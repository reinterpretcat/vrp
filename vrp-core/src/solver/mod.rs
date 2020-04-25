extern crate rand;
use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::common::{Cost, Objective};
use crate::models::{Problem, Solution};
use crate::solver::evolution::{run_evolution, EvolutionConfig};
use hashbrown::HashMap;
use std::any::Any;
use std::sync::Arc;

pub mod mutation;
pub mod objectives;
pub mod termination;

mod builder;
mod evolution;
mod population;

pub use self::builder::Builder;
pub use self::population::DominancePopulation;

/// Contains information needed to perform refinement.
pub struct RefinementContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Specifies solution population.
    pub population: Box<dyn Population + Sync + Send>,

    /// A collection of data associated with refinement process.
    pub state: HashMap<String, Box<dyn Any>>,

    /// A quota for refinement process.
    pub quota: Option<Box<dyn Quota + Send + Sync>>,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

/// Represents solution in population defined as actual solution.
pub type Individual = InsertionContext;

/// Represents a solution population.
pub trait Population {
    /// Adds individual into population.
    fn add(&mut self, individual: Individual);

    /// Returns all solutions from population sorted according their quality.
    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a>;

    /// Returns best solution from the population.
    fn best(&self) -> Option<&Individual>;

    /// Returns one of solutions from the population.
    fn select(&self) -> &Individual;

    /// Returns size of population.
    fn size(&self) -> usize;
}

impl RefinementContext {
    /// Creates a new instance of `[RefinementContext]`.
    pub fn new(
        problem: Arc<Problem>,
        population: Box<dyn Population + Sync + Send>,
        quota: Option<Box<dyn Quota + Send + Sync>>,
    ) -> Self {
        Self { problem, population, state: Default::default(), quota, generation: 1 }
    }
}

/// A logger type.
pub type Logger = Box<dyn Fn(String) -> ()>;

/// A Vehicle Routing Problem Solver.
pub struct Solver {
    pub problem: Arc<Problem>,
    pub config: EvolutionConfig,
}

impl Solver {
    pub fn solve(self) -> Result<(Solution, Cost), String> {
        let population = run_evolution(self.problem.clone(), self.config)?;

        // NOTE select first best according to population
        let insertion_ctx = population.best().ok_or_else(|| "cannot find any solution".to_string())?;
        let solution = insertion_ctx.solution.to_solution(self.problem.extras.clone());
        let cost = self.problem.objective.fitness(insertion_ctx);

        Ok((solution, cost))
    }
}
