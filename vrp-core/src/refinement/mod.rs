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

    /// A quota for refinement process.
    pub quota: Option<Box<dyn Quota + Send + Sync>>,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

/// Represents solution in population defined as actual solution, its cost, and generation
pub type Individuum = (InsertionContext, usize);

/// Represents a solution population.
pub trait Population {
    /// Adds individuum into population.
    fn add(&mut self, individuum: Individuum);

    /// Returns all solutions from population sorted according their quality.
    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individuum> + 'a>;

    /// Returns best solution from the population.
    fn best(&self) -> Option<&Individuum>;

    /// Returns one of solutions from the population.
    fn select(&self) -> &Individuum;

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

pub mod mutation;
pub mod objectives;
pub mod termination;
