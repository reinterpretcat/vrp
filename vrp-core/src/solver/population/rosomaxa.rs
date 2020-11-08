use super::*;
use crate::algorithms::gsom::{Input, Network, Storage};
use crate::models::Problem;
use std::sync::Arc;

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// Maps And eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct RosomaxaPopulation {
    problem: Arc<Problem>,
    elite: DominancePopulation,
    network: Network<IndividualInput, IndividualStorage>,
}

impl Population for RosomaxaPopulation {
    fn add_all(&mut self, individuals: Vec<Individual>, statistics: &Statistics) -> bool {
        individuals.into_iter().fold(false, |acc, individual| acc || self.add(individual, statistics))
    }

    fn add(&mut self, individual: Individual, statistics: &Statistics) -> bool {
        let is_improvement =
            if self.is_improvement(&individual) { self.elite.add(individual.deep_copy(), statistics) } else { false };

        // TODO use statistics to control network parameters
        self.network.train(IndividualInput { individual });

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        self.elite.select()
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        self.elite.ranked()
    }

    fn size(&self) -> usize {
        self.elite.size()
    }
}

impl RosomaxaPopulation {
    /// Creates a new instance of `RosomaxaPopulation`.
    pub fn new() -> Self {
        unimplemented!()
    }

    fn is_improvement(&self, individual: &Individual) -> bool {
        if let Some((best, _)) = self.elite.ranked().next() {
            if self.elite.cmp(individual, best) != Ordering::Greater {
                return !is_same_fitness(individual, best, self.problem.objective.as_ref());
            }
        } else {
            return true;
        }

        false
    }
}

struct IndividualInput {
    individual: InsertionContext,
}

impl Input for IndividualInput {
    fn weights(&self) -> &[f64] {
        unimplemented!()
    }
}

struct IndividualStorage {
    population: DominancePopulation,
}

impl Storage for IndividualStorage {
    type Item = IndividualInput;

    fn add(&mut self, input: Self::Item) {
        self.population.add(input.individual, &Statistics::default());
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.population.drain().into_iter().map(|individual| IndividualInput { individual }).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        unimplemented!()
    }
}
