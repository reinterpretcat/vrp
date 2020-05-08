#[cfg(test)]
#[path = "../../tests/unit/solver/population/population_test.rs"]
mod population_test;

use crate::algorithms::nsga2::{select_and_rank, Objective};
use crate::models::Problem;
use crate::solver::{Individual, Population};
use crate::utils::{compare_floats, Random};
use hashbrown::HashSet;
use std::cmp::Ordering::Equal;
use std::sync::Arc;

/// An evolution aware implementation of `[Population]` trait.
pub struct DominancePopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    individuals: Vec<Individual>,
    weights: Vec<usize>,
    offspring_size: usize,
    population_size: usize,
}

impl DominancePopulation {
    /// Creates a new instance of `[EvoPopulation]`.
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
                .enumerate()
                .map(|(idx, acd)| {
                    (
                        idx,
                        acd.index,
                        acd.crowding_distance,
                        self.problem.objective.fitness(self.individuals.get(acd.index).unwrap()),
                    )
                })
                .collect::<Vec<_>>();

        // sort population according to best order
        (0..self.individuals.len()).for_each(|i| loop {
            let (_, j, _, _) = best_order[i];
            let (_, k, _, _) = best_order[j];

            if i == j {
                break;
            }

            self.individuals.swap(j, k);
            best_order.swap(i, j);
        });

        // restore original order
        best_order.sort_by(|a, b| a.0.cmp(&b.0));

        // deduplicate best order
        best_order.dedup_by(|(_, _, a_cd, a_cost), (_, _, b_cd, b_cost)| {
            compare_floats(*a_cd, *b_cd) == Equal && compare_floats(*a_cost, *b_cost) == Equal
        });

        // deduplicate population
        let indices = best_order.iter().map(|i| i.0).collect::<HashSet<_>>();
        let mut idx = 0_usize;
        self.individuals.retain(|_| {
            idx += 1;
            indices.contains(&(idx - 1))
        });

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

    fn select(&self) -> &Individual {
        let idx = self.random.weighted(&self.weights[0..self.individuals.len()]);

        self.individuals.get(idx).unwrap()
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }
}
