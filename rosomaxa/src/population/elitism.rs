#[cfg(test)]
#[path = "../../tests/unit/population/elitism_test.rs"]
mod elitism_test;

use super::*;
use crate::algorithms::nsga2::select_and_rank;
use crate::utils::Random;
use crate::{HeuristicSpeed, HeuristicStatistics};
use std::cmp::Ordering;
use std::fmt::{Formatter, Write};
use std::iter::{empty, once};
use std::ops::RangeBounds;
use std::sync::Arc;

/// A simple evolution aware implementation of [`Population`] trait with the the following
/// characteristics:
///
/// - sorting of individuals in population according their objective fitness using [`NSGA-II`] algorithm
/// - maintaining diversity of population based on their crowding distance
///
/// [`Population`]: ./trait.Population.html
/// [`NSGA-II`]: ../algorithms/nsga2/index.html
///
pub struct Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    objective: Arc<O>,
    random: Arc<dyn Random + Send + Sync>,
    selection_size: usize,
    max_population_size: usize,
    individuals: Vec<S>,
    speed: Option<HeuristicSpeed>,
}

/// Keeps track of dominance order in the population for certain individual.
pub trait DominanceOrdered {
    /// Gets dominance order in the population.
    fn get_order(&self) -> &DominanceOrder;
    /// Sets dominance order in the population.
    fn set_order(&mut self, order: DominanceOrder);
}

/// Provides way to get a new objective by shuffling existing one.
pub trait Shuffled {
    /// Returns a new objective.
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self;
}

/// Contains ordering information about individual in population.
#[derive(Clone, Debug, Default)]
pub struct DominanceOrder {
    orig_index: usize,
    seq_index: usize,
    rank: usize,
}

impl<O, S> HeuristicPopulation for Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        if individuals.is_empty() {
            return false;
        }

        let was_empty = self.size() == 0;

        individuals.into_iter().for_each(|individual| {
            self.individuals.push(individual);
        });

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(was_empty)
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        let was_empty = self.size() == 0;

        self.individuals.push(individual);

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(was_empty)
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.speed = Some(statistics.speed.clone());
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.objective.total_order(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        let selection_size = match &self.speed {
            Some(HeuristicSpeed::Slow { ratio, .. }) => (self.selection_size as f64 * ratio).max(1.).round() as usize,
            _ => self.selection_size,
        };

        if self.individuals.is_empty() {
            Box::new(empty())
        } else {
            Box::new(
                once(0_usize)
                    .chain(
                        (1..selection_size).map(move |_| self.random.uniform_int(0, self.size() as i32 - 1) as usize),
                    )
                    .take(selection_size)
                    .filter_map(move |idx| self.individuals.get(idx)),
            )
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Individual, usize)> + 'a> {
        Box::new(self.individuals.iter().map(|individual| (individual, individual.get_order().rank)))
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.individuals.iter())
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }

    fn selection_phase(&self) -> SelectionPhase {
        SelectionPhase::Exploitation
    }
}

impl<O, S> Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    /// Creates a new instance of `Elitism`.
    ///
    /// * `problem` - a Vehicle Routing Problem definition.
    /// * `max_population_size` - a max size of population size.
    pub fn new(
        objective: Arc<O>,
        random: Arc<dyn Random + Send + Sync>,
        max_population_size: usize,
        selection_size: usize,
    ) -> Self {
        assert!(max_population_size > 0);

        Self { objective, random, selection_size, max_population_size, individuals: vec![], speed: None }
    }

    /// Shuffles objective function.
    pub fn shuffle_objective(&mut self) {
        self.objective = Arc::new(self.objective.get_shuffled(self.random.as_ref()));
    }

    /// Extracts all individuals from population.
    pub fn drain<R>(&mut self, range: R) -> Vec<S>
    where
        R: RangeBounds<usize>,
    {
        self.individuals.drain(range).collect()
    }

    fn sort(&mut self) {
        let objective = self.objective.clone();

        // get best order
        let best_order = select_and_rank(self.individuals.as_slice(), self.individuals.len(), objective.as_ref())
            .into_iter()
            .zip(0..)
            .map(|(acc, idx)| DominanceOrder { orig_index: acc.index, seq_index: idx, rank: acc.rank })
            .collect::<Vec<_>>();

        assert_eq!(self.individuals.len(), best_order.len());

        best_order.into_iter().for_each(|order| self.individuals[order.orig_index].set_order(order));
        self.individuals.sort_by(|a, b| a.get_order().seq_index.cmp(&b.get_order().seq_index));

        // deduplicate population
        self.individuals.dedup_by(|a, b| {
            if a.get_order().rank == b.get_order().rank {
                // NOTE just using crowding distance here does not work

                let fitness_a = a.get_fitness();
                let fitness_b = b.get_fitness();

                fitness_a.zip(fitness_b).all(|(a, b)| compare_floats(a, b) == Ordering::Equal)
            } else {
                false
            }
        });
    }

    fn ensure_max_population_size(&mut self) {
        if self.individuals.len() > self.max_population_size {
            self.individuals.truncate(self.max_population_size);
        }
    }

    fn is_improved(&self, was_empty: bool) -> bool {
        was_empty
            || self
                .individuals
                .first()
                .map(|individual| individual.get_order())
                .map_or(false, |dominance_order| dominance_order.orig_index != dominance_order.seq_index)
    }
}

impl<O, S> Display for Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fitness = self.individuals.iter().fold(String::new(), |mut res, individual| {
            let values = individual.get_fitness().map(|v| format!("{:.7}", v)).collect::<Vec<_>>().join(",");
            write!(&mut res, "[{}],", values).unwrap();

            res
        });

        write!(f, "[{}]", fitness)
    }
}
