use super::*;
use crate::algorithms::gsom::{Input, Network, Storage};
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::solver::SOLUTION_WEIGHTS_KEY;
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

        let weights = self.get_weights(&individual);
        self.network.train(IndividualInput::new_with_weights(individual, weights));

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        // TODO return individuals from elite (exploitation) and network (exploration)
        //      use statistics from add to control exploitation vs exploration balance
        unimplemented!()
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

    fn get_weights(&self, individual: &InsertionContext) -> Vec<f64> {
        vec![
            get_max_load_variance(individual),
            get_customers_deviation(individual),
            get_duration_mean(individual),
            get_distance_mean(individual),
            get_distance_gravity_mean(individual),
        ]
    }
}

struct IndividualInput {
    individual: InsertionContext,
}

impl IndividualInput {
    pub fn new_with_weights(individual: InsertionContext, weights: Vec<f64>) -> Self {
        let mut individual = individual;
        individual.solution.state.insert(SOLUTION_WEIGHTS_KEY, Arc::new(weights));

        Self { individual }
    }
}

impl Input for IndividualInput {
    fn weights(&self) -> &[f64] {
        self.individual
            .solution
            .state
            .get(&SOLUTION_WEIGHTS_KEY)
            .and_then(|s| s.downcast_ref::<Vec<f64>>())
            .unwrap()
            .as_slice()
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
        // NOTE as weights are not normalized, apply standardization using relative change: D = |x - y| / max(|x|, |y|)
        a.iter()
            .zip(b.iter())
            .fold(0., |acc, (a, b)| {
                let divider = a.abs().max(b.abs());
                let change = if compare_floats(divider, 0.) == Ordering::Equal { 0. } else { (a - b) / divider };

                acc + change * change
            })
            .sqrt()
    }
}
