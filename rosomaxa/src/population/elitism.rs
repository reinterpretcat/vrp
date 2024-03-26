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

/// A function type to deduplicate individuals.
pub type DedupFn<O, S> = Box<dyn Fn(&O, &S, &S) -> bool + Send + Sync>;

/// A simple evolution aware implementation of `Population` trait with the the following
/// characteristics:
///
/// - sorting of individuals in population according their objective fitness using `NSGA-II` algorithm
/// - maintaining diversity of population based on their crowding distance
///
pub struct Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    objective: Arc<O>,
    random: Random,
    selection_size: usize,
    max_population_size: usize,
    individuals: Vec<S>,
    speed: Option<HeuristicSpeed>,
    dedup_fn: DedupFn<O, S>,
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
    fn get_shuffled(&self, random: &Random) -> Self;
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

        self.add_with_iter(individuals.into_iter())
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.add_with_iter(once(individual))
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
    pub fn new(objective: Arc<O>, random: Random, max_population_size: usize, selection_size: usize) -> Self {
        Self::new_with_dedup(
            objective,
            random,
            max_population_size,
            selection_size,
            Box::new(|_, a, b| {
                if a.get_order().rank == b.get_order().rank {
                    // NOTE just using crowding distance here does not work

                    let fitness_a = a.fitness();
                    let fitness_b = b.fitness();

                    fitness_a.zip(fitness_b).all(|(a, b)| compare_floats(a, b) == Ordering::Equal)
                } else {
                    false
                }
            }),
        )
    }

    fn add_with_iter<I>(&mut self, iter: I) -> bool
    where
        I: Iterator<Item = S>,
    {
        let best_known_fitness = self.individuals.first().map(|i| i.fitness().collect());

        self.individuals.extend(iter);

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(best_known_fitness)
    }

    /// Creates a new instance of `Elitism` with custom deduplication function.
    pub fn new_with_dedup(
        objective: Arc<O>,
        random: Random,
        max_population_size: usize,
        selection_size: usize,
        dedup_fn: DedupFn<O, S>,
    ) -> Self {
        assert!(max_population_size > 0);
        Self { objective, random, selection_size, max_population_size, individuals: vec![], speed: None, dedup_fn }
    }

    /// Shuffles objective function.
    pub fn shuffle_objective(&mut self) {
        self.objective = Arc::new(self.objective.get_shuffled(&self.random));
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
        self.individuals.dedup_by(|a, b| (self.dedup_fn)(&objective, a, b));
    }

    fn ensure_max_population_size(&mut self) {
        if self.individuals.len() > self.max_population_size {
            self.individuals.truncate(self.max_population_size);
        }
    }

    fn is_improved(&self, best_known_fitness: Option<Vec<f64>>) -> bool {
        best_known_fitness.zip(self.individuals.first()).map_or(true, |(best_known_fitness, new_best_known)| {
            let dominance_order = new_best_known.get_order();
            if dominance_order.orig_index != dominance_order.seq_index {
                // NOTE: search is unstable, need to check fitness values
                best_known_fitness
                    .into_iter()
                    .zip(new_best_known.fitness())
                    .any(|(a, b)| compare_floats(a, b) != Ordering::Equal)
            } else {
                false
            }
        })
    }
}

impl<O, S> Display for Elitism<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + DominanceOrdered,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fitness = self.individuals.iter().fold(String::new(), |mut res, individual| {
            let values = individual.fitness().map(|v| format!("{v:.7}")).collect::<Vec<_>>().join(",");
            write!(&mut res, "[{values}],").unwrap();

            res
        });

        write!(f, "[{fitness}]")
    }
}
