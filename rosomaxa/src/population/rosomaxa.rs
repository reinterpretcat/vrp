#[cfg(test)]
#[path = "../../tests/unit/population/rosomaxa_test.rs"]
mod rosomaxa_test;

use super::*;
use crate::algorithms::gsom::*;
use crate::algorithms::math::relative_distance;
use crate::population::elitism::{DedupFn, Shuffled};
use crate::utils::{Environment, Random};
use rand::prelude::SliceRandom;
use rayon::iter::Either;
use std::convert::TryInto;
use std::f64::consts::{E, PI};
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
/// `MAps` and `eXtrAs` (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct Rosomaxa<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
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
    S: HeuristicSolution + RosomaxaWeighted,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        // NOTE avoid extra deep copy
        let best_known = self.elite.ranked().next();
        let elite = individuals
            .iter()
            .filter(|individual| self.is_comparable_with_best_known(individual, best_known))
            .map(|individual| init_individual(individual.deep_copy()))
            .collect::<Vec<_>>();
        let is_improved = self.elite.add_all(elite);

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: known_individuals } => known_individuals.extend(individuals),
            RosomaxaPhases::Exploration { network, statistics, .. } => {
                network.store_batch(individuals, statistics.generation, init_individual);
            }
            RosomaxaPhases::Exploitation { .. } => {}
        }

        is_improved
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        let best_known = self.elite.ranked().next();
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
            RosomaxaPhases::Exploration { network, coordinates, selection_size, statistics, .. } => {
                let random = self.environment.random.as_ref();

                let (elite_explore_size, node_explore_size) = match *selection_size {
                    value if value > 6 => {
                        let ratio = statistics.improvement_1000_ratio;
                        let elite_exlr_prob = 1. - 1. / (1. + E.powf(-10. * (ratio - 0.166)));
                        let elite_size = (1..=2)
                            .fold(0, |acc, idx| acc + if random.is_hit(elite_exlr_prob / idx as f64) { 2 } else { 1 });

                        const NODE_EXPLORE_PROB: f64 = 0.1;
                        let node_size = if random.is_hit(NODE_EXPLORE_PROB) { 2 } else { 1 };

                        (elite_size, node_size)
                    }
                    _ => (1, 1),
                };

                Box::new(
                    self.elite
                        .select()
                        .take(elite_explore_size)
                        .chain(coordinates.iter().flat_map(move |coordinate| {
                            network
                                .find(coordinate)
                                .map(|node| Either::Left(node.storage.population.select().take(node_explore_size)))
                                .unwrap_or_else(|| Either::Right(std::iter::empty()))
                        }))
                        .take(*selection_size),
                )
            }
            RosomaxaPhases::Exploitation { selection_size } => Box::new(self.elite.select().take(*selection_size)),
            _ => Box::new(self.elite.select()),
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        self.elite.ranked()
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                Box::new(self.elite.all().chain(network.get_nodes().flat_map(|node| node.storage.population.all())))
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
    S: HeuristicSolution + RosomaxaWeighted,
{
    /// Creates a new instance of `Rosomaxa`.
    pub fn new(objective: Arc<O>, environment: Arc<Environment>, config: RosomaxaConfig) -> Result<Self, GenericError> {
        if config.elite_size < 1 || config.node_size < 1 || config.selection_size < 2 {
            return Err("Rosomaxa algorithm requires some parameters to be above thresholds".into());
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

                    let initial = std::mem::take(individuals);
                    let initial = initial.into_iter().map(init_individual).collect::<Vec<_>>();
                    initial.iter().for_each(|individual| network.store(individual.deep_copy(), 0));

                    // create gene pool to keep track of population progress
                    let gene_pool_size = self.config.selection_size.clamp(4, 8);
                    let gene_pool_selection_size = (gene_pool_size / 2).max(4);
                    let mut gene_pool = Elitism::new_with_dedup(
                        self.objective.clone(),
                        self.environment.random.clone(),
                        gene_pool_size,
                        gene_pool_selection_size,
                        create_dedup_fn(0.05),
                    );
                    gene_pool.add_all(initial);

                    self.phase = RosomaxaPhases::Exploration {
                        network,
                        gene_pool,
                        coordinates: vec![],
                        statistics: statistics.clone(),
                        selection_size,
                    };
                }
            }
            RosomaxaPhases::Exploration {
                network,
                gene_pool,
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

                    Self::reintroduce_gene_pool(network, &self.elite, gene_pool, statistics, &self.config);

                    Self::optimize_network(network, statistics, &self.config);

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

    fn reintroduce_gene_pool(
        network: &mut IndividualNetwork<O, S>,
        elite: &Elitism<O, S>,
        gene_pool: &mut Elitism<O, S>,
        statistics: &HeuristicStatistics,
        config: &RosomaxaConfig,
    ) {
        let frequency = match statistics.speed {
            HeuristicSpeed::Slow { .. } => config.rebalance_memory.min(10),
            _ => (config.rebalance_memory / 2).max(20),
        };

        if statistics.generation > 0 && statistics.generation % frequency == 0 {
            network.store_batch(
                gene_pool.select().map(|i| i.deep_copy()).collect(),
                statistics.generation,
                init_individual,
            );

            if let Some(best_known) = elite.select().next() {
                gene_pool.add(best_known.deep_copy());
            }
        }
    }

    fn optimize_network(
        network: &mut IndividualNetwork<O, S>,
        statistics: &HeuristicStatistics,
        config: &RosomaxaConfig,
    ) {
        network.set_learning_rate(get_learning_rate(statistics.termination_estimate));

        if statistics.generation % config.rebalance_memory == 0 {
            network.smooth(1);
        }

        let keep_size = get_keep_size(config.rebalance_memory, statistics.termination_estimate);
        // no need to shrink network
        if network.size() <= keep_size {
            return;
        }

        network.compact();
        network.smooth(1);
    }

    fn fill_populations(network: &IndividualNetwork<O, S>, coordinates: &mut Vec<Coordinate>, random: &(dyn Random)) {
        coordinates.clear();
        coordinates.extend(network.iter().filter_map(|(coordinate, node)| {
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

        let storage_factory =
            IndividualStorageFactory { node_size: config.node_size, random: environment.random.clone(), objective };

        Network::new(
            *inputs_array,
            NetworkConfig {
                spread_factor: config.spread_factor,
                distribution_factor: config.distribution_factor,
                learning_rate: 0.1,
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
    S: HeuristicSolution + RosomaxaWeighted,
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
    S: HeuristicSolution + RosomaxaWeighted,
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
    S: HeuristicSolution + RosomaxaWeighted,
{
    Initial {
        solutions: Vec<S>,
    },
    Exploration {
        network: IndividualNetwork<O, S>,
        gene_pool: Elitism<O, S>,
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
    S: HeuristicSolution + RosomaxaWeighted,
{
    let mut individual = individual;
    individual.init_weights();

    individual
}

struct IndividualStorageFactory<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
{
    node_size: usize,
    random: Arc<dyn Random>,
    objective: Arc<O>,
}

impl<O, S> StorageFactory<S, IndividualStorage<O, S>> for IndividualStorageFactory<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
{
    fn eval(&self) -> IndividualStorage<O, S> {
        let mut elitism = Elitism::new_with_dedup(
            self.objective.clone(),
            self.random.clone(),
            self.node_size,
            self.node_size,
            create_dedup_fn(0.1),
        );

        elitism.shuffle_objective();

        IndividualStorage { population: elitism }
    }
}

struct IndividualStorage<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
{
    population: Elitism<O, S>,
}

impl<O, S> Storage for IndividualStorage<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
{
    type Item = S;

    fn add(&mut self, input: Self::Item) {
        self.population.add(input);
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Item> + 'a> {
        Box::new(self.population.ranked())
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
    S: HeuristicSolution + RosomaxaWeighted,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population)
    }
}

fn create_dedup_fn<O, S>(threshold: f64) -> DedupFn<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted,
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

/// Gets network size to keep.
/// Slowly decrease size of network from `3 * rebalance_memory` to `rebalance_memory`.
fn get_keep_size(rebalance_memory: usize, termination_estimate: f64) -> usize {
    let termination_estimate = termination_estimate.clamp(0., 0.8);
    // Sigmoid: https://www.wolframalpha.com/input?i=plot+1+*+%281%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let rate = 1. / (1. + E.powf(-10. * (termination_estimate - 0.5)));
    let keep_ratio = 2. * (1. - rate);

    rebalance_memory + (rebalance_memory as f64 * keep_ratio) as usize
}

/// Gets learning rate decay using cosine annealing.
/// `Cosine Annealing` is a type of learning rate schedule that has the effect of starting with a large
/// learning rate that is relatively rapidly decreased to a minimum value before being increased rapidly again.
fn get_learning_rate(termination_estimate: f64) -> f64 {
    const PERIOD: f64 = 0.25;
    const MIN_LEARNING_RATE: f64 = 0.1;
    const MAX_LEARNING_RATE: f64 = 1.0;

    assert!((0. ..=1.).contains(&termination_estimate), "termination estimate must be in [0, 1]");

    let min_lr = MIN_LEARNING_RATE;
    let max_lr = MAX_LEARNING_RATE;

    let progress = termination_estimate % PERIOD;
    let progress = progress / PERIOD;

    min_lr + 0.5 * (max_lr - min_lr) * (1. + (progress * PI).cos())
}
