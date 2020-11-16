//! Specifies population types.

mod dominance;
pub use self::dominance::DominancePopulation;

mod rosomaxa;
pub use self::rosomaxa::RosomaxaConfig;
pub use self::rosomaxa::RosomaxaPopulation;

use crate::construction::heuristics::InsertionContext;
use crate::solver::Statistics;
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::fmt::Display;

/// Represents solution in population defined as actual solution.
pub type Individual = InsertionContext;

/// Specifies a selection phase.
pub enum SelectionPhase {
    /// A phase of building an initial solution(-s).
    Initial,
    /// A phase of exploring solution space.
    Exploration,
    /// A phase of exploiting a region near best known optimum.
    Exploitation,
}

/// A trait which models a population with individuals (solutions).
pub trait Population: Display {
    /// Adds all individuals into the population, then sorts and shrinks population if necessary.
    /// Returns true if any of newly added individuals is considered as best known.
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool;

    /// Adds an individual into the population.
    /// Returns true if newly added individual is considered as best known.
    fn add(&mut self, individual: Individual) -> bool;

    /// Informs population about new generation event. This is time for the population
    /// to decide whether selection phase has to be changed.
    fn on_generation(&mut self, statistics: &Statistics);

    /// Compares two solutions the same way as population does.
    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering;

    /// Selects parents from the population based on current selection phase.
    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a>;

    /// Returns subset of individuals within their rank sorted according their quality.
    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a>;

    /// Returns population size.
    fn size(&self) -> usize;

    /// Returns a current selection phase.
    fn selection_phase(&self) -> SelectionPhase;
}

/// Checks whether two individuals have the same fitness values.
fn is_same_fitness(a: &Individual, b: &Individual) -> bool {
    let fitness_a = a.get_fitness_values();
    let fitness_b = b.get_fitness_values();

    fitness_a.zip(fitness_b).all(|(a, b)| compare_floats(a, b) == Ordering::Equal)
}
