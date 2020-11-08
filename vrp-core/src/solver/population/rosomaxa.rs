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
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool {
        unimplemented!()
    }

    fn add(&mut self, individual: Individual) -> bool {
        let is_improvement =
            if self.is_improvement(&individual) { self.elite.add(individual.deep_copy()) } else { false };

        self.network.train(IndividualInput { individual });

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        unimplemented!()
    }

    fn select(&self, statistics: &Statistics) -> Box<dyn Iterator<Item = &Individual>> {
        unimplemented!()
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)>> {
        unimplemented!()
    }

    fn size(&self) -> usize {
        unimplemented!()
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
        self.population.add(input.individual);
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.population.drain().into_iter().map(|individual| IndividualInput { individual }).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        unimplemented!()
    }
}
