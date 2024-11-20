#[cfg(test)]
#[path = "../../tests/unit/population/rosomaxa_test.rs"]
mod rosomaxa_test;

use super::*;
use crate::algorithms::gsom::*;
use crate::algorithms::math::relative_distance;
use crate::population::elitism::{DedupFn, Shuffled};
use crate::utils::{parallel_into_collect, Environment, Random};
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
    pub spread_factor: Float,
    /// Distribution factor of GSOM.
    pub distribution_factor: Float,
    /// A node rebalance memory of GSOM.
    pub rebalance_memory: usize,
    /// A ratio of exploration phase.
    pub exploration_ratio: Float,
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
pub trait RosomaxaSolution: HeuristicSolution + Input {
    /// An external context which is used within solutions.
    type Context: RosomaxaContext;

    /// Run on solution initialization. A time to update rosomaxa weights.
    fn on_init(&mut self, context: &Self::Context);
}

/// Specifies external context which can be used to analyze population evolution outside the algorithm.
pub trait RosomaxaContext: Send + Sync {
    /// A type of solution used within the context.
    type Solution: HeuristicSolution;

    /// A callback which is run on receiving a new solution set.
    fn on_change(&mut self, solutions: &[Self::Solution]);
}

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// `MAps` and `eXtrAs` (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct Rosomaxa<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    external_ctx: C,
    objective: Arc<O>,
    environment: Arc<Environment>,
    config: RosomaxaConfig,
    elite: Elitism<O, S>,
    phase: RosomaxaPhases<C, O, S>,
}

impl<C, O, S> HeuristicPopulation for Rosomaxa<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, mut individuals: Vec<Self::Individual>) -> bool {
        // NOTE avoid extra deep copy
        let best_known = self.elite.ranked().next();
        let elite = individuals
            .iter()
            .filter(|individual| self.is_comparable_with_best_known(individual, best_known))
            .map(|individual| init_individual(&self.external_ctx, individual.deep_copy()))
            .collect::<Vec<_>>();
        let is_improved = self.elite.add_all(elite);

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: known_individuals } => {
                self.external_ctx.on_change(individuals.as_mut_slice());
                known_individuals.extend(individuals)
            }
            RosomaxaPhases::Exploration { network, statistics, .. } => {
                self.external_ctx.on_change(individuals.as_mut_slice());
                network.store_batch(&self.external_ctx, individuals, statistics.generation, |i| {
                    init_individual(&self.external_ctx, i)
                });
            }
            RosomaxaPhases::Exploitation { .. } => {}
        }

        is_improved
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.add_all(vec![individual])
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
                        let elite_exlr_prob = (1. - 1. / (1. + E.powf(-10. * (ratio - 0.166)))) as Float;
                        let elite_size = (1..=2).fold(0, |acc, idx| {
                            acc + if random.is_hit(elite_exlr_prob / idx as Float) { 2 } else { 1 }
                        });

                        const NODE_EXPLORE_PROB: Float = 0.1;
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
            RosomaxaPhases::Exploitation { selection_size, .. } => Box::new(self.elite.select().take(*selection_size)),
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

type IndividualNetwork<C, O, S> = Network<C, S, IndividualStorage<C, O, S>, IndividualStorageFactory<C, O, S>>;

impl<C, O, S> Rosomaxa<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    /// Creates a new instance of `Rosomaxa`.
    pub fn new(
        external_ctx: C,
        objective: Arc<O>,
        environment: Arc<Environment>,
        config: RosomaxaConfig,
    ) -> Result<Self, GenericError> {
        if config.elite_size < 1 || config.node_size < 1 || config.selection_size < 2 {
            return Err("Rosomaxa algorithm requires some parameters to be above thresholds".into());
        }

        Ok(Self {
            external_ctx,
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
            HeuristicSpeed::Slow { ratio, .. } => {
                (self.config.selection_size as Float * ratio).max(1.).round() as usize
            }
        };

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: individuals } => {
                if individuals.len() >= 4 {
                    let mut network = Self::create_network(
                        &self.external_ctx,
                        self.objective.clone(),
                        self.environment.clone(),
                        &self.config,
                        individuals.drain(0..4).collect(),
                    );

                    std::mem::take(individuals).into_iter().for_each(|individual| {
                        network.store(&self.external_ctx, init_individual(&self.external_ctx, individual), 0)
                    });

                    self.phase = RosomaxaPhases::Exploration {
                        network,
                        coordinates: vec![],
                        statistics: statistics.clone(),
                        selection_size,
                    };
                }
            }
            RosomaxaPhases::Exploration {
                network,
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

                    Self::optimize_network(&self.external_ctx, network, statistics, &self.config);

                    Self::fill_populations(network, coordinates, self.environment.random.as_ref());
                } else {
                    self.phase = RosomaxaPhases::Exploitation { selection_size }
                }
            }
            RosomaxaPhases::Exploitation { selection_size: old_selection_size, .. } => {
                // NOTE as we exploit elite only, limit how many solutions are exploited simultaneously
                *old_selection_size = ((*old_selection_size as f64 / 2.).round() as usize).clamp(2, 4)
            }
        }
    }

    fn is_comparable_with_best_known(&self, individual: &S, best_known: Option<&S>) -> bool {
        best_known.map_or(true, |best_known| self.objective.total_order(individual, best_known) != Ordering::Greater)
    }

    fn optimize_network(
        external_ctx: &C,
        network: &mut IndividualNetwork<C, O, S>,
        statistics: &HeuristicStatistics,
        config: &RosomaxaConfig,
    ) {
        network.set_learning_rate(get_learning_rate(statistics.termination_estimate));

        if statistics.generation % config.rebalance_memory == 0 {
            network.smooth(external_ctx, 1);
        }

        let keep_size = get_keep_size(config.rebalance_memory, statistics.termination_estimate);
        // no need to shrink network
        if network.size() <= keep_size {
            return;
        }

        network.compact(external_ctx);
        network.smooth(external_ctx, 1);
    }

    fn fill_populations(
        network: &IndividualNetwork<C, O, S>,
        coordinates: &mut Vec<Coordinate>,
        random: &(dyn Random),
    ) {
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
        context: &C,
        objective: Arc<O>,
        environment: Arc<Environment>,
        config: &RosomaxaConfig,
        individuals: Vec<S>,
    ) -> IndividualNetwork<C, O, S> {
        let inputs_vec = parallel_into_collect(individuals, |i| init_individual(context, i));

        let inputs_slice = inputs_vec.into_boxed_slice();
        let inputs_array: Box<[S; 4]> = match inputs_slice.try_into() {
            Ok(ba) => ba,
            Err(o) => panic!("expected individuals of length {} but it was {}", 4, o.len()),
        };

        let storage_factory =
            IndividualStorageFactory { node_size: config.node_size, random: environment.random.clone(), objective };

        Network::new(
            context,
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

impl<'a, C, O, S> TryFrom<&'a Rosomaxa<C, O, S>> for NetworkState
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    type Error = String;

    fn try_from(value: &'a Rosomaxa<C, O, S>) -> Result<Self, Self::Error> {
        match &value.phase {
            RosomaxaPhases::Exploration { network, .. } => Ok(get_network_state(network)),
            _ => Err("not in exploration state".to_string()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum RosomaxaPhases<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    Initial {
        solutions: Vec<S>,
    },
    Exploration {
        network: IndividualNetwork<C, O, S>,
        coordinates: Vec<Coordinate>,
        statistics: HeuristicStatistics,
        selection_size: usize,
    },
    Exploitation {
        selection_size: usize,
    },
}

fn init_individual<C, S>(external_ctx: &C, individual: S) -> S
where
    C: RosomaxaContext<Solution = S>,
    S: RosomaxaSolution<Context = C>,
{
    let mut individual = individual;
    individual.on_init(external_ctx);

    individual
}

struct IndividualStorageFactory<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    node_size: usize,
    random: Arc<dyn Random>,
    objective: Arc<O>,
}

impl<C, O, S> StorageFactory<C, S, IndividualStorage<C, O, S>> for IndividualStorageFactory<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    fn eval(&self, _: &C) -> IndividualStorage<C, O, S> {
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

struct IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    population: Elitism<O, S>,
}

impl<C, O, S> Storage for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
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

    fn distance(&self, a: &[Float], b: &[Float]) -> Float {
        relative_distance(a.iter().cloned(), b.iter().cloned())
    }

    fn size(&self) -> usize {
        self.population.size()
    }
}

impl<C, O, S> Display for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population)
    }
}

fn create_dedup_fn<C, O, S>(threshold: Float) -> DedupFn<O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: RosomaxaSolution<Context = C>,
{
    // NOTE custom dedup rule to increase diversity property
    Box::new(move |objective, a, b| match objective.total_order(a, b) {
        Ordering::Equal => {
            let fitness_a = a.fitness();
            let fitness_b = b.fitness();

            fitness_a.zip(fitness_b).all(|(a, b)| a == b)
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
fn get_keep_size(rebalance_memory: usize, termination_estimate: Float) -> usize {
    #![allow(clippy::unnecessary_cast)]
    let termination_estimate = termination_estimate.clamp(0., 0.8) as f64;
    // Sigmoid: https://www.wolframalpha.com/input?i=plot+1+*+%281%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let rate = 1. / (1. + E.powf(-10. * (termination_estimate - 0.5)));
    let keep_ratio = 2. * (1. - rate);

    rebalance_memory + (rebalance_memory as Float * keep_ratio as Float) as usize
}

/// Gets learning rate decay using cosine annealing.
/// `Cosine Annealing` is a type of learning rate schedule that has the effect of starting with a large
/// learning rate that is relatively rapidly decreased to a minimum value before being increased rapidly again.
fn get_learning_rate(termination_estimate: Float) -> Float {
    #![allow(clippy::unnecessary_cast)]

    const PERIOD: Float = 0.25;
    const MIN_LEARNING_RATE: Float = 0.1;
    const MAX_LEARNING_RATE: Float = 1.0;

    assert!((0. ..=1.).contains(&termination_estimate), "termination estimate must be in [0, 1]");

    let min_lr = MIN_LEARNING_RATE;
    let max_lr = MAX_LEARNING_RATE;

    let progress = termination_estimate % PERIOD;
    let progress = progress / PERIOD;
    let progress_pi = (progress as f64 * PI) as Float;

    min_lr + 0.5 * (max_lr - min_lr) * (1. + progress_pi.cos())
}
