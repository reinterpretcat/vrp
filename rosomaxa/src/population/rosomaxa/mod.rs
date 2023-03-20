#[cfg(test)]
#[path = "../../../tests/unit/population/rosomaxa/rosomaxa_test.rs"]
mod rosomaxa_test;

mod optimizations;

use super::*;
use crate::algorithms::gsom::*;
use crate::algorithms::math::relative_distance;
use crate::population::elitism::{DedupFn, DominanceOrdered, Shuffled};
use crate::population::rosomaxa::optimizations::NetworkOptimization;
use crate::utils::{Environment, Random};
use rand::prelude::SliceRandom;
use std::convert::TryInto;
use std::fmt::Formatter;
use std::ops::RangeBounds;
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
            spread_factor: 0.75,
            distribution_factor: 0.75,
            objective_reshuffling: 0.01,
            learning_rate: 0.1,
            rebalance_memory: 100,
            exploration_ratio: 0.9,
        }
    }
}

/// Specifies behavior which keeps track of weights used to distinguish different solutions.
pub trait RosomaxaWeighted: Input {
    /// Initializes weights.
    fn init_weights(&mut self);
}

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// MAps and eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct Rosomaxa<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    objective: Arc<O>,
    environment: Arc<Environment>,
    config: RosomaxaConfig,
    elite: Elitism<O, S>,
    phase: RosomaxaPhases<O, S>,
}

impl<O, S> HeuristicPopulation for Rosomaxa<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        // NOTE avoid extra deep copy
        let best_known = self.elite.ranked().map(|(i, _)| i).next();
        let elite = individuals
            .iter()
            .filter(|individual| self.is_comparable_with_best_known(individual, best_known))
            .map(|individual| init_individual(individual.deep_copy()))
            .collect::<Vec<_>>();
        let is_improved = self.elite.add_all(elite);

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: known_individuals } => {
                known_individuals.extend(individuals.into_iter())
            }
            RosomaxaPhases::Exploration { network, statistics, .. } => {
                network.store_batch(individuals, statistics.generation, init_individual);
            }
            RosomaxaPhases::Exploitation { .. } => {}
        }

        is_improved
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        let best_known = self.elite.ranked().map(|(i, _)| i).next();
        let individual = init_individual(individual);
        let is_improved = if self.is_comparable_with_best_known(&individual, best_known) {
            self.elite.add(individual.deep_copy())
        } else {
            false
        };

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: individuals } => individuals.push(individual),
            RosomaxaPhases::Exploration { network, statistics, .. } => network.store(individual, statistics.generation),
            RosomaxaPhases::Exploitation { .. } => {}
        }

        is_improved
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.update_phase(statistics)
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, coordinates, selection_size, .. } => {
                let (elite_explore_size, node_explore_size) = match *selection_size {
                    value if value > 6 => {
                        let elite_size = self.environment.random.uniform_int(1, 2) as usize;
                        (elite_size, 2)
                    }
                    value if value > 4 => (1, 2),
                    _ => (1, 1),
                };

                Box::new(
                    self.elite
                        .select()
                        .take(elite_explore_size)
                        .chain(coordinates.iter().flat_map(move |coordinate| {
                            let explore_size = self.environment.random.uniform_int(1, node_explore_size) as usize;

                            network
                                .find(coordinate)
                                .map(|node| {
                                    let node = node.read().unwrap();
                                    // NOTE this is black magic to trick borrow checker, it should be safe to do
                                    // TODO is there better way to achieve similar result?
                                    unsafe { &*(&node.storage.population as *const Elitism<O, S>) as &Elitism<O, S> }
                                        .select()
                                        .take(explore_size)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_else(Vec::new)
                                .into_iter()
                        }))
                        .take(*selection_size),
                )
            }
            RosomaxaPhases::Exploitation { selection_size } => Box::new(self.elite.select().take(*selection_size)),
            _ => Box::new(self.elite.select()),
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Individual, usize)> + 'a> {
        self.elite.ranked()
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                Box::new(self.elite.all().chain(network.get_nodes().flat_map(|node| {
                    // NOTE see above
                    let node = node.read().unwrap();
                    unsafe { &*(&node.storage.population as *const Elitism<O, S>) as &Elitism<O, S> }.all()
                })))
            }
            _ => self.elite.all(),
        }
    }

    fn size(&self) -> usize {
        self.elite.size()
    }

    fn selection_phase(&self) -> SelectionPhase {
        match &self.phase {
            RosomaxaPhases::Initial { .. } => SelectionPhase::Initial,
            RosomaxaPhases::Exploration { .. } => SelectionPhase::Exploration,
            RosomaxaPhases::Exploitation { .. } => SelectionPhase::Exploitation,
        }
    }
}

type IndividualNetwork<O, S> = Network<S, IndividualStorage<O, S>, IndividualStorageFactory<O, S>>;

impl<O, S> Rosomaxa<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    /// Creates a new instance of `Rosomaxa`.
    pub fn new(objective: Arc<O>, environment: Arc<Environment>, config: RosomaxaConfig) -> Result<Self, String> {
        if config.elite_size < 1 || config.node_size < 1 || config.selection_size < 2 {
            return Err("Rosomaxa algorithm requires some parameters to be above thresholds".to_string());
        }

        Ok(Self {
            objective: objective.clone(),
            environment: environment.clone(),
            elite: Elitism::new_with_dedup(
                objective,
                environment.random.clone(),
                config.elite_size,
                config.selection_size,
                create_dedup_fn(0.02),
            ),
            phase: RosomaxaPhases::Initial { solutions: vec![] },
            config,
        })
    }

    fn update_phase(&mut self, statistics: &HeuristicStatistics) {
        let selection_size = match statistics.speed {
            HeuristicSpeed::Unknown | HeuristicSpeed::Moderate { .. } => self.config.selection_size,
            HeuristicSpeed::Slow { ratio, .. } => (self.config.selection_size as f64 * ratio).max(1.).round() as usize,
        };

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: individuals, .. } => {
                if individuals.len() >= 4 {
                    let mut network = Self::create_network(
                        self.objective.clone(),
                        self.environment.clone(),
                        &self.config,
                        individuals.drain(0..4).collect(),
                    );
                    individuals.drain(0..).for_each(|individual| network.store(init_individual(individual), 0));

                    self.phase = RosomaxaPhases::Exploration {
                        network,
                        optimization: NetworkOptimization::new(self.config.rebalance_memory, self.config.learning_rate),
                        coordinates: vec![],
                        statistics: statistics.clone(),
                        selection_size,
                    };
                }
            }
            RosomaxaPhases::Exploration {
                network,
                optimization,
                coordinates,
                statistics: old_statistics,
                selection_size: old_selection_size,
            } => {
                let exploration_ratio = match old_statistics.speed {
                    HeuristicSpeed::Unknown | HeuristicSpeed::Moderate { .. } => self.config.exploration_ratio,
                    HeuristicSpeed::Slow { ratio, .. } => self.config.exploration_ratio * ratio,
                };

                if statistics.termination_estimate < exploration_ratio {
                    *old_statistics = statistics.clone();
                    *old_selection_size = selection_size;
                    optimization.optimize_network(network, statistics);

                    Self::fill_populations(network, coordinates, self.environment.random.as_ref());
                } else {
                    self.phase = RosomaxaPhases::Exploitation { selection_size }
                }
            }
            RosomaxaPhases::Exploitation { selection_size: old_selection_size } => {
                *old_selection_size = selection_size;
            }
        }
    }

    fn is_comparable_with_best_known(&self, individual: &S, best_known: Option<&S>) -> bool {
        best_known.map_or(true, |best_known| self.objective.total_order(individual, best_known) != Ordering::Greater)
    }

    fn fill_populations(
        network: &IndividualNetwork<O, S>,
        coordinates: &mut Vec<Coordinate>,
        random: &(dyn Random + Send + Sync),
    ) {
        coordinates.clear();
        coordinates.extend(network.iter().filter_map(|(coordinate, node)| {
            let node = node.read().unwrap();
            if node.storage.population.size() > 0 {
                Some(*coordinate)
            } else {
                None
            }
        }));

        coordinates.shuffle(&mut random.get_rng());
    }

    fn create_network(
        objective: Arc<O>,
        environment: Arc<Environment>,
        config: &RosomaxaConfig,
        individuals: Vec<S>,
    ) -> IndividualNetwork<O, S> {
        let inputs_vec = individuals.into_iter().map(init_individual).collect::<Vec<_>>();

        let inputs_slice = inputs_vec.into_boxed_slice();
        let inputs_array: Box<[S; 4]> = match inputs_slice.try_into() {
            Ok(ba) => ba,
            Err(o) => panic!("expected individuals of length {} but it was {}", 4, o.len()),
        };

        let storage_factory = IndividualStorageFactory {
            node_size: config.node_size,
            reshuffling_probability: config.objective_reshuffling,
            random: environment.random.clone(),
            objective,
        };

        Network::new(
            *inputs_array,
            NetworkConfig {
                spread_factor: config.spread_factor,
                distribution_factor: config.distribution_factor,
                learning_rate: config.learning_rate,
                rebalance_memory: config.rebalance_memory,
                has_initial_error: true,
            },
            environment.random.clone(),
            storage_factory,
        )
    }
}

impl<O, S> Display for Rosomaxa<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                let state = get_network_state(network);
                write!(f, "{state}")
            }
            _ => write!(f, "{}", self.elite),
        }
    }
}

impl<'a, O, S> TryFrom<&'a Rosomaxa<O, S>> for NetworkState
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    type Error = String;

    fn try_from(value: &'a Rosomaxa<O, S>) -> Result<Self, Self::Error> {
        match &value.phase {
            RosomaxaPhases::Exploration { network, .. } => Ok(get_network_state(network)),
            _ => Err("not in exploration state".to_string()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum RosomaxaPhases<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    Initial {
        solutions: Vec<S>,
    },
    Exploration {
        network: IndividualNetwork<O, S>,
        optimization: NetworkOptimization<O, S>,
        coordinates: Vec<Coordinate>,
        statistics: HeuristicStatistics,
        selection_size: usize,
    },
    Exploitation {
        selection_size: usize,
    },
}

fn init_individual<S>(individual: S) -> S
where
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    let mut individual = individual;
    individual.init_weights();

    individual
}

struct IndividualStorageFactory<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    node_size: usize,
    reshuffling_probability: f64,
    random: Arc<dyn Random + Send + Sync>,
    objective: Arc<O>,
}

impl<O, S> StorageFactory<S, IndividualStorage<O, S>> for IndividualStorageFactory<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    fn eval(&self) -> IndividualStorage<O, S> {
        let mut elitism = Elitism::new_with_dedup(
            self.objective.clone(),
            self.random.clone(),
            self.node_size,
            self.node_size,
            create_dedup_fn(0.1),
        );
        if self.random.is_hit(self.reshuffling_probability) {
            elitism.shuffle_objective();
        }
        IndividualStorage { population: elitism }
    }
}

struct IndividualStorage<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    population: Elitism<O, S>,
}

impl<O, S> Storage for IndividualStorage<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    type Item = S;

    fn add(&mut self, input: Self::Item) {
        self.population.add(input);
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.population.ranked().map(|(r, _)| r))
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        self.population.drain(range).into_iter().collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        relative_distance(a.iter().cloned(), b.iter().cloned())
    }

    fn size(&self) -> usize {
        self.population.size()
    }
}

impl<O, S> Display for IndividualStorage<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population)
    }
}

fn create_dedup_fn<O, S>(threshold: f64) -> DedupFn<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    // NOTE custom dedup rule to increase diversity property
    Box::new(move |objective, a, b| match objective.total_order(a, b) {
        Ordering::Equal => {
            let fitness_a = a.fitness();
            let fitness_b = b.fitness();

            fitness_a.zip(fitness_b).all(|(a, b)| compare_floats(a, b) == Ordering::Equal)
        }
        _ => {
            let weights_a = a.weights();
            let weights_b = b.weights();
            let distance = relative_distance(weights_a.iter().cloned(), weights_b.iter().cloned());

            distance < threshold
        }
    })
}
