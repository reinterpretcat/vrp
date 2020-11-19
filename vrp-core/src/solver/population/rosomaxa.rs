use super::super::rand::prelude::SliceRandom;
use super::*;
use crate::algorithms::gsom::{get_network_state, Input, Network, NodeLink, Storage};
use crate::algorithms::statistics::relative_distance;
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::utils::{as_mut, get_cpus, Random};
use std::convert::TryInto;
use std::fmt::Formatter;
use std::ops::Deref;
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
    /// The reduction factor of GSOM.
    pub reduction_factor: f64,
    /// Distribution factor of GSOM.
    pub distribution_factor: f64,
    /// Learning rate of GSOM.
    pub learning_rate: f64,
    /// A node rebalance memory of GSOM.
    pub rebalance_memory: usize,
    /// A rebalance count.
    pub rebalance_count: usize,
    /// A ratio of exploration phase.
    pub exploration_ratio: f64,
}

impl Default for RosomaxaConfig {
    fn default() -> Self {
        Self {
            selection_size: get_cpus(),
            elite_size: 2,
            node_size: 2,
            spread_factor: 0.25,
            reduction_factor: 0.1,
            distribution_factor: 0.25,
            learning_rate: 0.1,
            rebalance_memory: 500,
            rebalance_count: 10,
            exploration_ratio: 0.9,
        }
    }
}

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// MAps and eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct Rosomaxa {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    config: RosomaxaConfig,
    elite: Elitism,
    phase: RosomaxaPhases,
}

impl Population for Rosomaxa {
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool {
        individuals.into_iter().fold(false, |acc, individual| acc || self.add_individual(individual))
    }

    fn add(&mut self, individual: Individual) -> bool {
        self.add_individual(individual)
    }

    fn on_generation(&mut self, statistics: &Statistics) {
        self.update_phase(statistics)
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        // NOTE we always promote 2 elements from elite and 2 from each population in the network
        //      in exploring phase. 2 is not a magic number: dominance population always promotes
        //      the best individual as first, all others are selected with equal probability.
        match &self.phase {
            RosomaxaPhases::Exploration { populations, .. } => Box::new(
                self.elite
                    .select()
                    .take(2)
                    .chain(populations.iter().flat_map(|population| population.0.select().take(2)))
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
    pub fn new(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        config: RosomaxaConfig,
    ) -> Result<Self, ()> {
        if config.elite_size < 2 || config.node_size < 2 || config.selection_size < 4 {
            return Err(());
        }

        Ok(Self {
            problem: problem.clone(),
            random: random.clone(),
            elite: Elitism::new(problem.clone(), random.clone(), config.elite_size, config.selection_size),
            phase: RosomaxaPhases::Initial { individuals: vec![] },
            config,
        })
    }

    /// Creates a new instance of `Rosomaxa` or `Elitism` if config is too restrictive.
    pub fn new_with_fallback(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        config: RosomaxaConfig,
    ) -> Box<dyn Population + Send + Sync> {
        let selection_size = config.selection_size;
        let max_population_size = config.elite_size;

        Rosomaxa::new(problem.clone(), random.clone(), config)
            .map::<Box<dyn Population + Send + Sync>, _>(|population| Box::new(population))
            .unwrap_or_else(|()| Box::new(Elitism::new(problem, random, max_population_size, selection_size)))
    }

    fn add_individual(&mut self, individual: Individual) -> bool {
        match &mut self.phase {
            RosomaxaPhases::Initial { individuals } => {
                if individuals.len() < 4 {
                    individuals.push(individual.deep_copy());
                }
            }
            RosomaxaPhases::Exploration { time, network, .. } => {
                network.store(IndividualInput::new(individual.deep_copy()), *time);
            }
            RosomaxaPhases::Exploitation => {}
        };

        if self.is_improvement(&individual) {
            self.elite.add(individual)
        } else {
            false
        }
    }

    fn update_phase(&mut self, statistics: &Statistics) {
        match &mut self.phase {
            RosomaxaPhases::Initial { individuals, .. } => {
                if individuals.len() >= 4 {
                    let mut network = Self::create_network(
                        self.problem.clone(),
                        self.random.clone(),
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
                    let is_optimization_time = *time % self.config.rebalance_memory == 0;

                    if is_optimization_time {
                        Self::optimize_network(
                            network,
                            best_fitness.as_slice(),
                            statistics,
                            self.config.rebalance_count,
                        )
                    }

                    Self::fill_populations(
                        network,
                        populations,
                        best_fitness.as_slice(),
                        self.random.as_ref(),
                        statistics,
                    );
                } else {
                    self.phase = RosomaxaPhases::Exploitation
                }
            }
            RosomaxaPhases::Exploitation => {}
        }
    }

    fn is_improvement(&self, individual: &Individual) -> bool {
        if let Some((best, _)) = self.elite.ranked().next() {
            if self.elite.cmp(individual, best) != Ordering::Greater {
                return !is_same_fitness(individual, best);
            }
        } else {
            return true;
        }

        false
    }

    fn fill_populations(
        network: &IndividualNetwork,
        populations: &mut Vec<(Arc<Elitism>, f64)>,
        best_fitness: &[f64],
        random: &(dyn Random + Send + Sync),
        statistics: &Statistics,
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

        populations.sort_by(|(_, a), (_, b)| compare_floats(*a, *b));

        // NOTE we keep track of actual populations and randomized order to keep selection algorithm simple
        let shuffle_ratio = (1. - statistics.termination_estimate).min(0.5).max(1.).round();
        let shuffle_amount = (populations.len() as f64 * shuffle_ratio) as usize;
        populations.partial_shuffle(&mut random.get_rng(), shuffle_amount);
    }

    fn optimize_network(
        network: &mut IndividualNetwork,
        best_fitness: &[f64],
        statistics: &Statistics,
        rebalance_count: usize,
    ) {
        let percentile_threshold = 0.25 * statistics.termination_estimate.min(0.25).max(1.0);

        let get_distance = |node: &NodeLink<IndividualInput, IndividualStorage>| {
            let node = node.read().unwrap();
            let individual = node.storage.population.select().next();
            if let Some(individual) = individual {
                Some(relative_distance(best_fitness.iter().cloned(), individual.get_fitness_values()))
            } else {
                None
            }
        };

        // determine percentile value
        let mut distances = network.get_nodes().filter_map(get_distance).collect::<Vec<_>>();
        distances.sort_by(|a, b| compare_floats(*b, *a));
        let percentile_idx = (distances.len() as f64 * percentile_threshold) as usize;

        if let Some(distance_threshold) = distances.get(percentile_idx).cloned() {
            network.optimize(rebalance_count, &|node| {
                let is_empty = node.read().unwrap().storage.population.size() == 0;

                is_empty || get_distance(node).map_or(true, |distance| distance > distance_threshold)
            });
        }
    }

    fn create_network(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
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
            config.spread_factor,
            config.reduction_factor,
            config.distribution_factor,
            config.learning_rate,
            config.rebalance_memory,
            Box::new({
                let problem = problem.clone();
                let random = random.clone();
                let node_size = config.node_size;
                move || IndividualStorage {
                    population: Arc::new(Elitism::new(problem.clone(), random.clone(), node_size, node_size)),
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
        Self { individual, weights }
    }

    fn get_weights(individual: &InsertionContext) -> Vec<f64> {
        vec![
            get_max_load_variance(individual),
            get_customers_deviation(individual),
            get_duration_mean(individual),
            get_distance_mean(individual),
            get_waiting_mean(individual),
            get_distance_gravity_mean(individual),
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

    fn drain(&mut self) -> Vec<Self::Item> {
        self.get_population_mut().drain().into_iter().map(IndividualInput::new).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        relative_distance(a.iter().cloned(), b.iter().cloned())
    }
}

impl Display for IndividualStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population.as_ref())
    }
}
