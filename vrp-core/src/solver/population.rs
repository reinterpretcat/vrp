#[cfg(test)]
#[path = "../../tests/unit/solver/population/population_test.rs"]
mod population_test;

use crate::algorithms::nsga2::{select_and_rank, Objective};
use crate::models::Problem;
use crate::solver::{Individual, Population};
use crate::utils::{compare_floats, Random};
use std::cmp::Ordering::Equal;
use std::sync::Arc;

/// A simple evolution aware implementation of [`Population`] trait with the the following
/// characteristics:
///
/// - sorting of individuals in population according their objective fitness using [`NSGA-II`] algorithm
/// - maintaining diversity of population based on their ranking
/// - individuals from elite group are more preferable for selection
///
/// [`Population`]: ./trait.Population.html
/// [`NSGA-II`]: ../algorithms/nsga2/index.html
///
pub struct DominancePopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    individuals: Vec<Individual>,
    weights: Vec<usize>,
    offspring_size: usize,
    population_size: usize,
}

impl DominancePopulation {
    /// Creates a new instance of `DominancePopulation`.
    ///
    /// * `problem` - a Vehicle Routing Problem definition.
    /// * `random` - an implementation of [`Random`](../utils/trait.Random.html) trait.
    /// * `population_size` - a max size of population without offspring.
    /// * `offspring_size` - a max size of offspring before shrinking occurs.
    /// * `elite_size` - amount of individuals considered as elite.
    ///
    pub fn new(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        population_size: usize,
        offspring_size: usize,
        elite_size: usize,
    ) -> Self {
        assert!(elite_size < population_size);

        let max_size = population_size + offspring_size;

        Self {
            problem,
            random,
            individuals: vec![],
            weights: (0..max_size)
                .map(|idx| {
                    let weight = max_size - idx;
                    weight + if idx < elite_size { weight } else { 0 }
                })
                .collect(),
            population_size,
            offspring_size,
        }
    }
}

impl Population for DominancePopulation {
    fn add(&mut self, individual: Individual) {
        self.individuals.push(individual);

        let max_size = self.population_size + self.offspring_size;

        // get best order
        let mut best_order =
            select_and_rank(self.individuals.as_slice(), self.individuals.len(), self.problem.objective.as_ref())
                .iter()
                .map(|acd| {
                    (
                        acd.index,
                        acd.crowding_distance,
                        self.problem.objective.fitness(self.individuals.get(acd.index).unwrap()),
                    )
                })
                .collect::<Vec<_>>();

        // TODO there seems to be bug in select_and_rank: empty collection can be returned
        if !best_order.is_empty() {
            // deduplicate best order
            best_order.dedup_by(|(_, a_cd, a_cost), (_, b_cd, b_cost)| {
                compare_floats(*a_cd, *b_cd) == Equal && compare_floats(*a_cost, *b_cost) == Equal
            });

            // TODO avoid deep copy
            self.individuals = best_order.iter().map(|(idx, _, _)| self.individuals[*idx].deep_copy()).collect();
        }

        if self.individuals.len() > max_size {
            self.individuals.truncate(self.population_size);
        }
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        Box::new(self.individuals.iter())
    }

    fn best(&self) -> Option<&Individual> {
        self.individuals.first()
    }

    fn select(&self) -> Option<&Individual> {
        if self.individuals.is_empty() {
            return None;
        }

        let idx = self.random.weighted(&self.weights[0..self.individuals.len()]);

        self.individuals.get(idx)
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }
}
