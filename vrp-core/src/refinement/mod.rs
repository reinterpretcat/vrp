//! Buildings blocks for metaheuristic (e.g. ruin and recreate, genetic, etc.).
//!
//! **Metaheuristic** is a higher-level procedure which tries to refine existing solution (e.g. found by
//! insertion heuristic) escaping local optimum.
//! One of metaheuristic examples, is **ruin and recreate**, formulated by
//! [Schrimpf et al. (2000)](https://www.sciencedirect.com/science/article/pii/S0021999199964136).
//! It describes approach which essentially destroys parts of solution and rebuild from it solution
//! with some modifications.

extern crate rand;

use crate::construction::heuristics::InsertionContext;
use crate::models::Problem;

use crate::construction::Quota;
use crate::refinement::objectives::ObjectiveCostType;
use hashbrown::HashMap;
use std::any::Any;
use std::sync::Arc;

/// Contains information needed to perform refinement.
pub struct RefinementContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Specifies solution population.
    pub population: Box<dyn Population + Sync + Send>,

    /// A collection of data associated with refinement process.
    pub state: HashMap<String, Box<dyn Any>>,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

/// Represents solution in population defined as actual solution, its cost, and generation
pub type Individuum = (InsertionContext, ObjectiveCostType, usize);

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
        Self::new_with_population(problem, Box::new(SinglePopulation::default()))
    }

    pub fn new_with_population(problem: Arc<Problem>, population: Box<dyn Population + Sync + Send>) -> Self {
        Self { problem, population, state: Default::default(), generation: 1 }
    }

    pub fn get_quota(&self) -> Option<&Box<dyn Quota + Send + Sync>> {
        self.state.get("quota").and_then(|q| q.downcast_ref::<Box<dyn Quota + Send + Sync>>())
    }

    pub fn set_quota(&mut self, quota: Box<dyn Quota + Send + Sync>) {
        self.state.insert("quota".to_string(), Box::new(quota));
    }
}

pub mod acceptance;
pub mod mutation;
pub mod objectives;
pub mod selection;
pub mod termination;

pub mod population;
