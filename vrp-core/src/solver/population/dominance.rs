#[cfg(test)]
#[path = "../../../tests/unit/solver/population/dominance_test.rs"]
mod dominance_test;

use super::*;
use crate::algorithms::nsga2::{select_and_rank, Objective};
use crate::models::Problem;
use crate::solver::{Population, Statistics, SOLUTION_ORDER_KEY};
use crate::utils::Random;
use std::cmp::Ordering;
use std::fmt::{Formatter, Write};
use std::iter::{empty, once};
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
pub struct DominancePopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    selection_size: usize,
    max_population_size: usize,
    individuals: Vec<Individual>,
}

/// Contains ordering information about individual in population.
#[derive(Clone, Debug)]
struct DominanceOrder {
    orig_index: usize,
    seq_index: usize,
    rank: usize,
    crowding_distance: f64,
}

impl Population for DominancePopulation {
    fn add_all(&mut self, individuals: Vec<Individual>, _: &Statistics) -> bool {
        let was_empty = self.size() == 0;

        individuals.into_iter().for_each(|individual| {
            self.individuals.push(individual);
        });

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(was_empty)
    }

    fn add(&mut self, individual: Individual, _: &Statistics) -> bool {
        let was_empty = self.size() == 0;

        self.individuals.push(individual);

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(was_empty)
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.problem.objective.total_order(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        if self.individuals.is_empty() {
            Box::new(empty())
        } else {
            Box::new(
                once(0_usize)
                    .chain(
                        (1..self.selection_size)
                            .map(move |_| self.random.uniform_int(0, self.size() as i32 - 1) as usize),
                    )
                    .take(self.selection_size)
                    .filter_map(move |idx| self.individuals.get(idx)),
            )
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        Box::new(self.individuals.iter().map(|individual| (individual, Self::gen_dominance_order(individual).rank)))
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }
}

impl DominancePopulation {
    /// Creates a new instance of `DominancePopulation`.
    ///
    /// * `problem` - a Vehicle Routing Problem definition.
    /// * `max_population_size` - a max size of population size.
    pub fn new(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        max_population_size: usize,
        selection_size: usize,
    ) -> Self {
        assert!(max_population_size > 0);

        Self { problem, random, selection_size, max_population_size, individuals: vec![] }
    }

    /// Extracts all individuals out from population.
    pub fn drain(&mut self) -> Vec<Individual> {
        std::mem::replace(&mut self.individuals, vec![])
    }

    fn sort(&mut self) {
        let objective = self.problem.objective.clone();

        // get best order
        let best_order = select_and_rank(self.individuals.as_slice(), self.individuals.len(), objective.as_ref())
            .into_iter()
            .zip(0..)
            .map(|(acc, idx)| DominanceOrder {
                orig_index: acc.index,
                seq_index: idx,
                rank: acc.rank,
                crowding_distance: acc.crowding_distance,
            })
            .collect::<Vec<_>>();

        assert_eq!(self.individuals.len(), best_order.len());

        // remember dominance order
        best_order.into_iter().for_each(|order| {
            self.individuals[order.orig_index].solution.state.insert(SOLUTION_ORDER_KEY, Arc::new(order));
        });

        // sort by best order
        self.individuals.sort_by(|a, b| {
            let a = Self::gen_dominance_order(a);
            let b = Self::gen_dominance_order(b);

            a.seq_index.cmp(&b.seq_index)
        });

        // deduplicate population
        self.individuals.dedup_by(|a, b| {
            let order_a = Self::gen_dominance_order(a);
            let order_b = Self::gen_dominance_order(b);

            if order_a.rank == order_b.rank {
                // NOTE just using crowding distance here does not work
                is_same_fitness(a, b)
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
                .map(Self::gen_dominance_order)
                .map_or(false, |dominance_order| dominance_order.orig_index != dominance_order.seq_index)
    }

    fn gen_dominance_order(individual: &Individual) -> &DominanceOrder {
        individual.solution.state.get(&SOLUTION_ORDER_KEY).and_then(|s| s.downcast_ref::<DominanceOrder>()).unwrap()
    }
}

impl Display for DominancePopulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fitness = self.individuals.iter().fold(String::new(), |mut res, individual| {
            let values = individual.get_fitness_values().map(|v| format!("{:.7}", v)).collect::<Vec<_>>().join(",");
            write!(&mut res, "[{}],", values).unwrap();

            res
        });

        write!(f, "[{}]", fitness)
    }
}
