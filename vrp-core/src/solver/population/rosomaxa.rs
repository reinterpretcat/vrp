use super::*;
use crate::algorithms::gsom::{Input, Network, Storage};

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// Maps And eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct RosomaxaPopulation {
    network: Network<IndividualInput, IndividualStorage>,
}

impl Population for RosomaxaPopulation {
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool {
        unimplemented!()
    }

    fn add(&mut self, individual: Individual) -> bool {
        unimplemented!()
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
        unimplemented!()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        unimplemented!()
    }
}

impl Default for IndividualStorage {
    fn default() -> Self {
        unimplemented!()
    }
}
