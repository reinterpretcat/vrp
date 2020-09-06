#[cfg(test)]
#[path = "../../tests/unit/solver/population/population_test.rs"]
mod population_test;

use crate::algorithms::nsga2::{select_and_rank, Objective};
use crate::models::Problem;
use crate::solver::{Individual, Population, SOLUTION_ORDER_KEY};
use crate::utils::compare_floats;
use std::cmp::Ordering;
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

impl DominancePopulation {
    /// Creates a new instance of `DominancePopulation`.
    ///
    /// * `problem` - a Vehicle Routing Problem definition.
    /// * `max_population_size` - a max size of population size.
    pub fn new(problem: Arc<Problem>, max_population_size: usize) -> Self {
        assert!(max_population_size > 0);

        Self { problem, max_population_size, individuals: vec![] }
    }
}

impl Population for DominancePopulation {
    fn add_all(&mut self, individuals: Vec<Individual>) {
        individuals.into_iter().for_each(|individual| {
            self.individuals.push(individual);
        });

        self.sort();
        self.ensure_max_population_size();
    }

    fn add(&mut self, individual: Individual) {
        self.individuals.push(individual);

        self.sort();
        self.ensure_max_population_size();
    }

    fn nth(&self, idx: usize) -> Option<&Individual> {
        self.individuals.get(idx)
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.problem.objective.total_order(a, b)
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        Box::new(self.individuals.iter().map(|individual| (individual, Self::gen_dominance_order(individual).rank)))
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }
}

impl DominancePopulation {
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
            let a = Self::gen_dominance_order(a);
            let b = Self::gen_dominance_order(b);

            a.rank == b.rank && compare_floats(a.crowding_distance, b.crowding_distance) == Ordering::Equal
        });
    }

    fn ensure_max_population_size(&mut self) {
        if self.individuals.len() > self.max_population_size {
            self.individuals.truncate(self.max_population_size);
        }
    }

    fn gen_dominance_order(individual: &Individual) -> &DominanceOrder {
        individual.solution.state.get(&SOLUTION_ORDER_KEY).and_then(|s| s.downcast_ref::<DominanceOrder>()).unwrap()
    }
}
