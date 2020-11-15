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
    /// A node hit memory of GSOM.
    pub hit_memory: usize,
}

impl Default for RosomaxaConfig {
    fn default() -> Self {
        Self {
            selection_size: get_cpus(),
            elite_size: 2,
            node_size: 2,
            spread_factor: 0.5,
            reduction_factor: 0.1,
            distribution_factor: 0.25,
            learning_rate: 0.1,
            hit_memory: 1000,
        }
    }
}

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// Maps And eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct RosomaxaPopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    config: RosomaxaConfig,
    elite: DominancePopulation,
    phase: RosomaxaPhases,
}

impl Population for RosomaxaPopulation {
    fn add_all(&mut self, individuals: Vec<Individual>, statistics: &Statistics) -> bool {
        let is_improvement =
            individuals.into_iter().fold(false, |acc, individual| acc || self.add_individual(individual, statistics));

        self.update_phase(statistics);

        is_improvement
    }

    fn add(&mut self, individual: Individual, statistics: &Statistics) -> bool {
        let is_improvement = self.add_individual(individual, statistics);

        self.update_phase(statistics);

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        // TODO return individuals from elite (exploitation) and network (exploration)
        //      use statistics from add to control exploitation vs exploration balance
        //      use hits to select candidates for mating depending on evolution progress (statistics)

        // NOTE we always promote 2 elements from elite and 2 from each population in the network
        //      in exploring phase. 2 is not a magic number: dominance population always promotes
        //      the best individual as first, all others are selected with equal probability.

        // TODO If calling site selects always less than 3 elements, then the algorithm should be
        //      adjusted to handle that. At the moment, idea is always promote elite in some degree.

        match &self.phase {
            RosomaxaPhases::Exploring { populations, .. } => Box::new(
                self.elite
                    .select()
                    .take(2)
                    .chain(populations.iter().flat_map(|population| population.select().take(2)))
                    .take(self.config.selection_size),
            ),
            _ => Box::new(self.elite.select()),
        }
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
        config: RosomaxaConfig,
    ) -> Result<Self, ()> {
        // NOTE see note at selection method implementation
        if config.elite_size < 2 || config.node_size < 2 || config.selection_size < 4 {
            return Err(());
        }

        Ok(Self {
            problem: problem.clone(),
            random: random.clone(),
            elite: DominancePopulation::new(problem.clone(), random.clone(), config.elite_size, config.selection_size),
            phase: RosomaxaPhases::Initial { individuals: vec![] },
            config,
        })
    }

    /// Creates a new instance of `RosomaxaPopulation` or `DominancePopulation` if
    /// settings does not allow.
    pub fn new_with_fallback(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        config: RosomaxaConfig,
    ) -> Box<dyn Population + Send + Sync> {
        let selection_size = config.selection_size;
        let max_population_size = config.elite_size;

        RosomaxaPopulation::new(problem.clone(), random.clone(), config)
            .map::<Box<dyn Population + Send + Sync>, _>(|population| Box::new(population))
            .unwrap_or_else(|()| {
                Box::new(DominancePopulation::new(problem, random, max_population_size, selection_size))
            })
    }

    fn add_individual(&mut self, individual: Individual, statistics: &Statistics) -> bool {
        match &mut self.phase {
            RosomaxaPhases::Initial { individuals } => {
                if individuals.len() < 4 {
                    individuals.push(individual.deep_copy());
                }
            }
            RosomaxaPhases::Exploring { network, .. } => {
                network.store(IndividualInput::new(individual.deep_copy()), statistics.generation);
            }
        };

        if self.is_improvement(&individual) {
            self.elite.add(individual.deep_copy(), statistics)
        } else {
            false
        }
    }

    fn update_phase(&mut self, statistics: &Statistics) {
        match &mut self.phase {
            RosomaxaPhases::Initial { individuals, .. } => {
                if individuals.len() >= 4 {
                    self.phase = RosomaxaPhases::Exploring {
                        network: Self::create_network(
                            self.problem.clone(),
                            self.random.clone(),
                            &self.config,
                            individuals.drain(0..).collect(),
                        ),
                        populations: vec![],
                    };
                }
            }
            RosomaxaPhases::Exploring { network, populations, .. } => {
                let best_individual = self.elite.select().next();
                let is_optimization_time = statistics.generation % self.config.hit_memory == 0;

                match (best_individual, is_optimization_time) {
                    (Some(best_individual), true) => {
                        // TODO determine parameters based on generation and configuration
                        const PERCENTILE_THRESHOLD: f64 = 0.25;
                        const REBALANCE_COUNT: usize = 10;

                        let best_fitness = best_individual.get_fitness_values().collect::<Vec<_>>();
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
                        let percentile_idx = (distances.len() as f64 * PERCENTILE_THRESHOLD) as usize;

                        if let Some(distance_threshold) = distances.get(percentile_idx).cloned() {
                            network.optimize(REBALANCE_COUNT, &|node| {
                                let is_empty = node.read().unwrap().storage.population.size() == 0;
                                is_empty || get_distance(node).map_or(true, |distance| distance > distance_threshold)
                            });
                        }
                    }
                    _ => {}
                }

                populations.clear();
                populations.extend(
                    network
                        .get_nodes()
                        .map(|node| node.read().unwrap().storage.population.clone())
                        .filter(|population| population.size() > 0),
                );

                // NOTE we keep track of actual populations and randomized order to keep selection algorithm simple
                populations.shuffle(&mut self.random.get_rng());
            }
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

    fn create_network(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        config: &RosomaxaConfig,
        individuals: Vec<Individual>,
    ) -> Network<IndividualInput, IndividualStorage> {
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
            config.hit_memory,
            Box::new({
                let problem = problem.clone();
                let random = random.clone();
                let node_size = config.node_size;
                move || IndividualStorage {
                    population: Arc::new(DominancePopulation::new(
                        problem.clone(),
                        random.clone(),
                        node_size,
                        node_size,
                    )),
                }
            }),
        )
    }
}

impl Display for RosomaxaPopulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.phase {
            RosomaxaPhases::Exploring { network, .. } => {
                let state = get_network_state(network);
                write!(f, "{}", state)
            }
            _ => write!(f, "{}", self.elite),
        }
    }
}

enum RosomaxaPhases {
    /// Collecting initial solutions phase.
    Initial { individuals: Vec<InsertionContext> },

    /// Exploring solution space phase.
    Exploring { network: Network<IndividualInput, IndividualStorage>, populations: Vec<Arc<DominancePopulation>> },
    // TODO add a phase for exploiting region with most promising optimum
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
    population: Arc<DominancePopulation>,
}

impl IndividualStorage {
    fn get_population_mut(&mut self) -> &mut DominancePopulation {
        // NOTE use black magic here to avoid RefCell, should not break memory safety guarantee
        unsafe { as_mut(self.population.deref()) }
    }
}

impl Storage for IndividualStorage {
    type Item = IndividualInput;

    fn add(&mut self, input: Self::Item) {
        self.get_population_mut().add(input.individual, &Statistics::default());
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
