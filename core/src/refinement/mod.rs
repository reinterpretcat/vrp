extern crate rand;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::Problem;

use std::sync::Arc;

/// Contains information needed to perform refinement.
pub struct RefinementContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Specifies solution population.
    pub population: Box<dyn Population>,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

/// Represents solution in population defined as actual solution, its cost, and generation
pub type Individuum = (InsertionContext, ObjectiveCost, usize);

/// Represents a solution population.
pub trait Population {
    /// Adds individuum into population.
    fn add(&mut self, individuum: Individuum);

    /// Returns all solutions from population sorted according their quality.
    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a>;

    /// Returns best solution from population.
    fn best(&self) -> Option<&Individuum>;

    /// Returns size of population.
    fn size(&self) -> usize;
}

/// A population which consist maximum of one solution.
struct SinglePopulation {
    individuums: Vec<Individuum>,
}

impl Default for SinglePopulation {
    fn default() -> Self {
        Self { individuums: vec![] }
    }
}

impl Population for SinglePopulation {
    fn add(&mut self, individuum: Individuum) {
        self.individuums.clear();
        self.individuums.push(individuum);
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a> {
        Box::new(self.individuums.iter())
    }

    fn best(&self) -> Option<&Individuum> {
        self.individuums.first()
    }

    fn size(&self) -> usize {
        self.individuums.len()
    }
}

impl RefinementContext {
    pub fn new(problem: Arc<Problem>) -> Self {
        Self { problem, population: Box::new(SinglePopulation::default()), generation: 1 }
    }

    pub fn new_with_population(problem: Arc<Problem>, population: Box<dyn Population>) -> Self {
        Self { problem, population, generation: 1 }
    }
}

pub mod acceptance;
pub mod objectives;
pub mod recreate;
pub mod ruin;
pub mod selection;
pub mod termination;
