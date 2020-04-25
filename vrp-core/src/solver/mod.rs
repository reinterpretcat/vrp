use crate::models::common::Cost;
use crate::models::{Problem, Solution};

mod builder;
pub use self::builder::Builder;
use crate::solver::evolution::{run_evolution, EvolutionConfig};
use std::sync::Arc;

mod evolution;
mod population;
pub use self::population::DominancePopulation;

mod sorting;

/// A logger type.
pub type Logger = Box<dyn Fn(String) -> ()>;

pub struct Solver {
    pub problem: Arc<Problem>,
    pub config: EvolutionConfig,
}

impl Solver {
    pub fn solve(self) -> Result<(Solution, Cost, usize), String> {
        let population = run_evolution(self.problem.clone(), self.config)?;

        // NOTE select first best according to population
        let (insertion_ctx, generation) = population.best().ok_or_else(|| "cannot find any solution".to_string())?;
        let solution = insertion_ctx.solution.to_solution(self.problem.extras.clone());
        let cost = self.problem.objective.fitness(insertion_ctx);

        Ok((solution, cost, *generation))
    }
}
