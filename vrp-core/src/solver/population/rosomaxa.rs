use super::*;
use crate::algorithms::gsom::{Input, Network, Storage};
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::solver::SOLUTION_WEIGHTS_KEY;
use crate::utils::Random;
use std::sync::Arc;

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// Maps And eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct RosomaxaPopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
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

        self.network.train(IndividualInput::new(individual));

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        // TODO return individuals from elite (exploitation) and network (exploration)
        //      use statistics from add to control exploitation vs exploration balance
        //      use hits to select candidates for mating depending on evolution progress (statistics)

        //self.network.nodes().

        unimplemented!()
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        // NOTE return only elite
        self.elite.ranked()
    }

    fn size(&self) -> usize {
        self.elite.size()
    }
}

impl RosomaxaPopulation {
    /// Creates a new instance of `RosomaxaPopulation`.
    pub fn new(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        initials: [InsertionContext; 4],
        max_elite_size: usize,
        max_node_size: usize,
        spread_factor: f64,
        reduction_factor: f64,
        distribution_factor: f64,
        learning_rate: f64,
    ) -> Self {
        let [a, b, c, d] = initials;

        Self {
            problem: problem.clone(),
            random: random.clone(),
            elite: DominancePopulation::new(problem.clone(), random.clone(), max_elite_size, max_elite_size),
            network: Network::new(
                [IndividualInput::new(a), IndividualInput::new(b), IndividualInput::new(c), IndividualInput::new(d)],
                spread_factor,
                reduction_factor,
                distribution_factor,
                learning_rate,
                Box::new(move || IndividualStorage {
                    population: DominancePopulation::new(problem.clone(), random.clone(), max_node_size, max_node_size),
                }),
            ),
        }
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

impl IndividualInput {
    pub fn new(individual: InsertionContext) -> Self {
        let weights = get_weights(&individual);

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

impl IndividualStorage {
    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        self.population.select()
    }
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

fn get_weights(individual: &InsertionContext) -> Vec<f64> {
    vec![
        get_max_load_variance(individual),
        get_customers_deviation(individual),
        get_duration_mean(individual),
        get_distance_mean(individual),
        get_distance_gravity_mean(individual),
    ]
}
