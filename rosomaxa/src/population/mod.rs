//! Specifies population types.

mod elitism;
pub use self::elitism::{Alternative, Elitism};

mod greedy;
pub use self::greedy::Greedy;

mod rosomaxa;
pub use self::rosomaxa::{Rosomaxa, RosomaxaConfig, RosomaxaContext, RosomaxaSolution};

use crate::prelude::*;
use std::cmp::Ordering;
use std::fmt::Display;

/// Specifies a selection phase.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SelectionPhase {
    /// A phase of building an initial solution(-s).
    Initial,
    /// A phase of exploring solution space.
    Exploration,
    /// A phase of exploiting a region near best known optimum.
    Exploitation,
}

/// A trait which models a population with individuals.
pub trait HeuristicPopulation: Send + Sync {
    /// A heuristic objective type.
    type Objective: HeuristicObjective;
    /// A solution type.
    type Individual: HeuristicSolution;

    /// Adds all individuals into the population, then sorts and shrinks population if necessary.
    /// Returns true if any of newly added individuals is considered as best known.
    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool;

    /// Adds an individual into the population.
    /// Returns true if newly added individual is considered as best known.
    fn add(&mut self, individual: Self::Individual) -> bool;

    /// Informs population about new generation event. This is time for the population
    /// to decide whether selection phase has to be changed.
    fn on_generation(&mut self, statistics: &HeuristicStatistics);

    /// Compares two solutions the same way as population does.
    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering;

    /// Selects parents from the population based on current selection phase.
    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a>;

    /// Returns subset of individuals within their rank sorted according their quality.
    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a>;

    /// Returns all individuals in arbitrary order.
    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a>;

    /// Returns population size.
    fn size(&self) -> usize;

    /// Returns a current selection phase.
    fn selection_phase(&self) -> SelectionPhase;
}
