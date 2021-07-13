#[cfg(test)]
#[path = "../../../tests/unit/solver/population/rosomaxa_test.rs"]
mod rosomaxa_test;

use super::super::rand::prelude::SliceRandom;
use super::*;
use crate::algorithms::gsom::{get_network_state, Input, Network, NetworkConfig, NodeLink, Storage};
use crate::algorithms::nsga2::Objective;
use crate::algorithms::statistics::relative_distance;
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::utils::{as_mut, Environment, Random};
use std::convert::TryInto;
use std::fmt::Formatter;
use std::ops::{Deref, RangeBounds};
use std::sync::Arc;

/// Specifies rosomaxa configuration settings.
pub struct RosomaxaConfig {
    /// Selection size.
    pub selection_size: usize,
    /// Elite population size.
    pub elite_size: usize,
    /// Node population size.
    pub node_size: usize,
    /// Spread factor of GSOM.
    pub spread_factor: f64,
    /// Distribution factor of GSOM.
    pub distribution_factor: f64,
    /// Objective reshuffling probability.
    pub objective_reshuffling: f64,
    /// Learning rate of GSOM.
    pub learning_rate: f64,
    /// A node rebalance memory of GSOM.
    pub rebalance_memory: usize,
    /// A rebalance count.
    pub rebalance_count: usize,
    /// A ratio of exploration phase.
    pub exploration_ratio: f64,
}

impl RosomaxaConfig {
    /// Creates an instance of `RosomaxaConfig` using default parameters, but taking into
    /// account data parallelism settings.
    pub fn new_with_defaults(selection_size: usize) -> Self {
        Self {
            selection_size,
            elite_size: 2,
            node_size: 2,
            spread_factor: 0.25,
            distribution_factor: 0.25,
            objective_reshuffling: 0.01,
            learning_rate: 0.1,
            rebalance_memory: 100,
            rebalance_count: 2,
            exploration_ratio: 0.9,
        }
    }
}

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// MAps and eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct Rosomaxa {
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    config: RosomaxaConfig,
    elite: Elitism,
    phase: RosomaxaPhases,
}

impl Population for Rosomaxa {
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool {
        // NOTE avoid extra deep copy
        let best_known = self.elite.ranked().map(|(i, _)| i).next();
        let elite = individuals
            .iter()
            .filter(|individual| self.is_comparable_with_best_known(individual, best_known))
            .map(|individual| individual.deep_copy())
            .collect::<Vec<_>>();
        let is_improved = self.elite.add_all(elite);

        match &mut self.phase {
            RosomaxaPhases::Initial { individuals: known_individuals } => {
                known_individuals.extend(individuals.into_iter())
            }
            RosomaxaPhases::Exploration { time, network, .. } => {
                network.store_batch(individuals, *time, IndividualInput::new);
            }
            RosomaxaPhases::Exploitation => {}
        }

        is_improved
    }

    fn add(&mut self, individual: Individual) -> bool {
        let best_known = self.elite.ranked().map(|(i, _)| i).next();
        let is_improved = if self.is_comparable_with_best_known(&individual, best_known) {
            self.elite.add(individual.deep_copy())
        } else {
            false
        };

        match &mut self.phase {
            RosomaxaPhases::Initial { individuals } => individuals.push(individual),
            RosomaxaPhases::Exploration { time, network, .. } => network.store(IndividualInput::new(individual), *time),
            RosomaxaPhases::Exploitation => {}
        }

        is_improved
    }

    fn on_generation(&mut self, statistics: &Statistics) {
        self.update_phase(statistics)
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        let (elite_explore_size, node_explore_size) = match self.config.selection_size {
            value if value > 6 => {
                let elite_size = self.environment.random.uniform_int(2, 4) as usize;
                (elite_size, 2)
            }
            value if value > 4 => (2, 2),
            value if value > 2 => (2, 1),
            _ => (1, 1),
        };

        match &self.phase {
            RosomaxaPhases::Exploration { populations, .. } => Box::new(
                self.elite
                    .select()
                    .take(elite_explore_size)
                    .chain(populations.iter().flat_map(move |population| {
                        let explore_size = self.environment.random.uniform_int(1, node_explore_size) as usize;
                        population.0.select().take(explore_size)
                    }))
                    .take(self.config.selection_size),
            ),
            _ => Box::new(self.elite.select()),
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        self.elite.ranked()
    }

    fn size(&self) -> usize {
        self.elite.size()
    }

    fn selection_phase(&self) -> SelectionPhase {
        match &self.phase {
            RosomaxaPhases::Initial { .. } => SelectionPhase::Initial,
            RosomaxaPhases::Exploration { .. } => SelectionPhase::Exploration,
            RosomaxaPhases::Exploitation => SelectionPhase::Exploitation,
        }
    }
}

type IndividualNetwork = Network<IndividualInput, IndividualStorage>;

impl Rosomaxa {
    /// Creates a new instance of `Rosomaxa`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>, config: RosomaxaConfig) -> Result<Self, String> {
        if config.elite_size < 2 || config.node_size < 2 || config.selection_size < 2 {
            return Err("Rosomaxa algorithm requires some parameters to be above thresholds".to_string());
        }

        Ok(Self {
            problem: problem.clone(),
            environment: environment.clone(),
            elite: Elitism::new(problem, environment.random.clone(), config.elite_size, config.selection_size),
            phase: RosomaxaPhases::Initial { individuals: vec![] },
            config,
        })
    }

    fn update_phase(&mut self, statistics: &Statistics) {
        match &mut self.phase {
            RosomaxaPhases::Initial { individuals, .. } => {
                if individuals.len() >= 4 {
                    let mut network = Self::create_network(
                        self.problem.clone(),
                        self.environment.clone(),
                        &self.config,
                        individuals.drain(0..4).collect(),
                    );
                    individuals.drain(0..).for_each(|individual| network.store(IndividualInput::new(individual), 0));

                    self.phase = RosomaxaPhases::Exploration { time: 0, network, populations: vec![] };
                }
            }
            RosomaxaPhases::Exploration { time, network, populations, .. } => {
                if statistics.termination_estimate < self.config.exploration_ratio {
                    *time = statistics.generation;
                    let best_individual = self.elite.select().next().expect("expected individuals in elite");
                    let best_fitness = best_individual.get_fitness_values().collect::<Vec<_>>();

                    Self::optimize_network(
                        network,
                        statistics,
                        best_fitness.as_slice(),
                        self.config.rebalance_memory,
                        self.config.rebalance_count,
                    );

                    Self::fill_populations(
                        network,
                        populations,
                        best_fitness.as_slice(),
                        statistics,
                        self.environment.random.as_ref(),
                    );
                } else {
                    self.phase = RosomaxaPhases::Exploitation
                }
            }
            RosomaxaPhases::Exploitation => {}
        }
    }

    fn is_comparable_with_best_known(&self, individual: &Individual, best_known: Option<&Individual>) -> bool {
        best_known
            .map_or(true, |best_known| self.problem.objective.total_order(&individual, best_known) != Ordering::Greater)
    }

    fn fill_populations(
        network: &IndividualNetwork,
        populations: &mut Vec<(Arc<Elitism>, f64)>,
        best_fitness: &[f64],
        statistics: &Statistics,
        random: &(dyn Random + Send + Sync),
    ) {
        populations.clear();
        populations.extend(network.get_nodes().map(|node| node.read().unwrap().storage.population.clone()).filter_map(
            |population| {
                population.select().next().map(|individual| {
                    (
                        population.clone(),
                        relative_distance(best_fitness.iter().cloned(), individual.get_fitness_values()),
                    )
                })
            },
        ));

        let shuffle_amount = Self::get_shuffle_amount(statistics, populations.len());

        if shuffle_amount != populations.len() {
            // partially randomize order
            populations.sort_by(|(_, a), (_, b)| compare_floats(*a, *b));
            populations.partial_shuffle(&mut random.get_rng(), shuffle_amount);
        } else {
            populations.shuffle(&mut random.get_rng());
        }
    }

    fn get_shuffle_amount(statistics: &Statistics, length: usize) -> usize {
        let ratio = match statistics.improvement_1000_ratio {
            v if v > 0.5 => {
                // https://www.wolframalpha.com/input/?i=plot+0.66+*+%281-+1%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
                let progress = statistics.termination_estimate;
                let ratio = 0.5 * (1. - 1. / (1. + std::f64::consts::E.powf(-10. * (progress - 0.5))));
                ratio.clamp(0.1, 0.5)
            }
            v if v > 0.2 => 0.5,
            _ => 1.,
        };

        (length as f64 * ratio).round() as usize
    }

    fn optimize_network(
        network: &mut IndividualNetwork,
        statistics: &Statistics,
        best_fitness: &[f64],
        rebalance_memory: usize,
        rebalance_count: usize,
    ) {
        let rebalance_memory = rebalance_memory as f64;
        let keep_size = match statistics.improvement_1000_ratio {
            v if v > 0.2 => {
                // https://www.wolframalpha.com/input/?i=plot+%281+-+1%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
                let x = statistics.termination_estimate.clamp(0., 1.);
                let ratio = 1. - 1. / (1. + std::f64::consts::E.powf(-10. * (x - 0.5)));
                rebalance_memory + rebalance_memory * ratio
            }
            v if v > 0.1 => 2. * rebalance_memory,
            v if v > 0.01 => 3. * rebalance_memory,
            _ => 4. * rebalance_memory,
        } as usize;

        if statistics.generation == 0 || network.size() <= keep_size {
            return;
        }

        let get_distance = |node: &NodeLink<IndividualInput, IndividualStorage>| {
            let node = node.read().unwrap();
            let individual = node.storage.population.select().next();

            individual
                .map(|individual| relative_distance(best_fitness.iter().cloned(), individual.get_fitness_values()))
        };

        // determine percentile value
        let mut distances = network.get_nodes().filter_map(get_distance).collect::<Vec<_>>();
        distances.sort_by(|a, b| compare_floats(*b, *a));
        let percentile_idx = if distances.len() > keep_size {
            distances.len() - keep_size
        } else {
            const PERCENTILE_THRESHOLD: f64 = 0.1;

            (distances.len() as f64 * PERCENTILE_THRESHOLD) as usize
        };

        if let Some(distance_threshold) = distances.get(percentile_idx).cloned() {
            network.retrain(rebalance_count, &|node| {
                get_distance(node).map_or(false, |distance| distance < distance_threshold)
            });
        }
    }

    fn create_network(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
        config: &RosomaxaConfig,
        individuals: Vec<Individual>,
    ) -> IndividualNetwork {
        let inputs_vec = individuals.into_iter().map(IndividualInput::new).collect::<Vec<_>>();

        let inputs_slice = inputs_vec.into_boxed_slice();
        let inputs_array: Box<[IndividualInput; 4]> = match inputs_slice.try_into() {
            Ok(ba) => ba,
            Err(o) => panic!("expected individuals of length {} but it was {}", 4, o.len()),
        };

        Network::new(
            *inputs_array,
            NetworkConfig {
                spread_factor: config.spread_factor,
                distribution_factor: config.distribution_factor,
                learning_rate: config.learning_rate,
                rebalance_memory: config.rebalance_memory,
            },
            Box::new({
                let node_size = config.node_size;
                let reshuffling_probability = config.objective_reshuffling;
                let random = environment.random.clone();

                move || {
                    let mut elitism = Elitism::new(problem.clone(), random.clone(), node_size, node_size);
                    if random.is_hit(reshuffling_probability) {
                        elitism.shuffle_objective();
                    }
                    IndividualStorage { population: Arc::new(elitism) }
                }
            }),
        )
    }
}

impl Display for Rosomaxa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                let state = get_network_state(network);
                write!(f, "{}", state)
            }
            _ => write!(f, "{}", self.elite),
        }
    }
}

enum RosomaxaPhases {
    Initial { individuals: Vec<InsertionContext> },
    Exploration { time: usize, network: IndividualNetwork, populations: Vec<(Arc<Elitism>, f64)> },
    Exploitation,
}

struct IndividualInput {
    weights: Vec<f64>,
    individual: InsertionContext,
}

impl IndividualInput {
    pub fn new(individual: InsertionContext) -> Self {
        let weights = IndividualInput::get_weights(&individual);
        Self { weights, individual }
    }

    fn get_weights(individual: &InsertionContext) -> Vec<f64> {
        vec![
            get_max_load_variance(individual),
            get_customers_deviation(individual),
            get_duration_mean(individual),
            get_distance_mean(individual),
            get_waiting_mean(individual),
            get_distance_gravity_mean(individual),
            individual.solution.routes.len() as f64,
        ]
    }
}

impl Input for IndividualInput {
    fn weights(&self) -> &[f64] {
        self.weights.as_slice()
    }
}

struct IndividualStorage {
    population: Arc<Elitism>,
}

impl IndividualStorage {
    fn get_population_mut(&mut self) -> &mut Elitism {
        // NOTE use black magic here to avoid RefCell, should not break memory safety guarantee
        unsafe { as_mut(self.population.deref()) }
    }
}

impl Storage for IndividualStorage {
    type Item = IndividualInput;

    fn add(&mut self, input: Self::Item) {
        self.get_population_mut().add(input.individual);
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        self.get_population_mut().drain(range).into_iter().map(IndividualInput::new).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        relative_distance(a.iter().cloned(), b.iter().cloned())
    }

    fn size(&self) -> usize {
        self.population.size()
    }
}

impl Display for IndividualStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population.as_ref())
    }
}
