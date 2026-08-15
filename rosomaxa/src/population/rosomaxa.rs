#[cfg(test)]
#[path = "../../tests/unit/population/rosomaxa_test.rs"]
mod rosomaxa_test;

use super::*;
use crate::algorithms::gsom::*;
use crate::algorithms::math::relative_distance;
use crate::population::elitism::{Alternative, DedupFn};
use crate::utils::{Environment, ParallelismScope, Random, parallel_into_collect};
use rand::prelude::SliceRandom;
use std::f64::consts::{E, PI};
use std::fmt::Formatter;
use std::ops::RangeBounds;
use std::sync::Arc;

/// Specifies rosomaxa configuration settings.
pub struct RosomaxaConfig {
    /// Number of candidates collected before GSOM initialization.
    pub initial_size: usize,
    /// Selection size.
    pub selection_size: usize,
    /// Elite population size.
    pub elite_size: usize,
    /// Maximum number of solutions retained by each GSOM node. Smoothing replays all retained solutions; parent
    /// selection takes one solution per node and normally favors its best one.
    pub node_size: usize,
    /// Spread factor of GSOM.
    pub spread_factor: Float,
    /// Distribution factor of GSOM.
    pub distribution_factor: Float,
    /// Maximum number of nodes retained by GSOM before compaction.
    pub max_network_size: usize,
    /// A ratio of exploration phase.
    pub exploration_ratio: Float,
}

impl RosomaxaConfig {
    /// Creates an instance of `RosomaxaConfig` using default parameters and the given selection size.
    pub fn new_with_defaults(selection_size: usize) -> Self {
        Self {
            initial_size: 32,
            selection_size,
            elite_size: 2,
            node_size: 2,
            spread_factor: 0.75,
            distribution_factor: 0.9,
            max_network_size: 600,
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

    /// Run on context update.
    fn on_update(&mut self, context: &Self::Context);
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
    O: HeuristicObjective<Solution = S> + Alternative,
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
    C: RosomaxaContext<Solution = S> + 'static,
    O: HeuristicObjective<Solution = S> + Alternative + 'static,
    S: RosomaxaSolution<Context = C> + 'static,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        // NOTE avoid extra deep copy
        let best_known = self.elite.best();
        let elite = individuals
            .iter()
            .filter(|individual| self.is_comparable_with_best_known(individual, best_known))
            .map(|individual| init_individual(&self.external_ctx, individual.deep_copy()))
            .collect::<Vec<_>>();
        let is_improved = self.elite.add_all(elite);

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: known_individuals } => {
                self.external_ctx.on_change(individuals.as_slice());
                known_individuals.extend(individuals)
            }
            RosomaxaPhases::Exploration { network, new_input_count, statistics, .. } => {
                self.external_ctx.on_change(individuals.as_slice());
                let data = parallel_into_collect(individuals, ParallelismScope::Local, |i| {
                    init_individual(&self.external_ctx, i)
                });
                *new_input_count = new_input_count.saturating_add(data.len());
                network.store_batch(&self.external_ctx, data, statistics.generation);
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

    fn select(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        match &self.phase {
            RosomaxaPhases::Initial { solutions } => {
                let mut parents = solutions.iter().collect::<Vec<_>>();
                parents.sort_by(|left, right| self.objective.total_order(left, right));
                parents.truncate(self.config.selection_size);

                Box::new(parents.into_iter())
            }
            RosomaxaPhases::Exploration { network, selection_coordinates, selection_size, statistics, .. } => {
                let random = self.environment.random.as_ref();
                let elite_selection_size =
                    get_elite_selection_size(*selection_size, statistics.improvement_1000_ratio, |probability| {
                        random.is_hit(probability)
                    });

                let node_alternative_probability = if *selection_size > 6 {
                    get_node_alternative_probability(statistics.termination_estimate)
                } else {
                    0.
                };

                Box::new(
                    self.elite
                        .select()
                        .take(elite_selection_size)
                        .chain(
                            selection_coordinates
                                .iter()
                                .filter_map(move |coordinate| network.find(coordinate))
                                .flat_map(move |node| node.storage.select(random, node_alternative_probability)),
                        )
                        // A small map might not have enough retained node solutions to fill a large selection budget.
                        .chain(self.elite.select())
                        .take(*selection_size),
                )
            }
            RosomaxaPhases::Exploitation { selection_size, .. } => Box::new(self.elite.select().take(*selection_size)),
        }
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.elite.ranked()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                Box::new(self.elite.iter().chain(network.iter_nodes().flat_map(|node| node.storage.population.iter())))
            }
            _ => self.elite.iter(),
        }
    }

    fn into_iter(self: Box<Self>) -> Box<dyn Iterator<Item = Self::Individual>> {
        match self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                Box::new(Box::new(self.elite).into_iter().chain(
                    network.into_iter_nodes().flat_map(|(_, node)| Box::new(node.storage.population).into_iter()),
                ))
            }
            _ => Box::new(self.elite).into_iter(),
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

// Hit history is exposed in GSOM state, but does not control its maintenance or capacity.
const HIT_MEMORY_SIZE: usize = 200;

// Let the young map grow before smoothing can reset the node errors which drive GSOM growth.
const INITIAL_GROWTH_OBSERVATION_WINDOWS: usize = 2;

// A larger candidate pool gives different constructors a chance to improve before GSOM is trained. Keep its input
// bounded to the previous default size so weak outliers do not increase training work or shape the whole map.
const INITIAL_NETWORK_SIZE: usize = 16;

impl<C, O, S> Rosomaxa<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    /// Creates a new instance of `Rosomaxa`.
    pub fn new(
        external_ctx: C,
        objective: Arc<O>,
        environment: Arc<Environment>,
        config: RosomaxaConfig,
    ) -> Result<Self, GenericError> {
        if config.initial_size < 1
            || config.elite_size < 1
            || config.node_size < 1
            || config.max_network_size < 4
            || config.selection_size < 2
        {
            return Err("Rosomaxa population and network sizes must be above their minimums".into());
        }
        if !(config.spread_factor > 0. && config.spread_factor < 1.)
            || !(config.distribution_factor > 0. && config.distribution_factor < 1.)
        {
            return Err("Rosomaxa spread and distribution factors must be finite and within (0, 1)".into());
        }
        if !(config.exploration_ratio >= 0. && config.exploration_ratio <= 1.) {
            return Err("Rosomaxa exploration ratio must be finite and within [0, 1]".into());
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

        let exploration_ratio = get_exploration_ratio(self.config.exploration_ratio, statistics.improvement_1000_ratio);

        match &mut self.phase {
            RosomaxaPhases::Initial { solutions: individuals } => {
                if statistics.termination_estimate >= exploration_ratio {
                    (self.environment.logger)("skip exploration phase");
                    self.phase = RosomaxaPhases::Exploitation { selection_size }
                } else if individuals.len() >= self.config.initial_size {
                    let network_result = Self::create_network(
                        &self.external_ctx,
                        self.objective.clone(),
                        self.environment.clone(),
                        &self.config,
                        std::mem::take(individuals),
                    );

                    match network_result {
                        Ok(network) => {
                            let selection_coordinates = network.get_coordinates().collect();

                            self.phase = RosomaxaPhases::Exploration {
                                network,
                                new_input_count: 0,
                                is_network_warmed_up: false,
                                selection_coordinates,
                                local_optimum_candidates: Vec::new(),
                                statistics: statistics.clone(),
                                selection_size,
                            };
                        }
                        Err(err) => {
                            (self.environment.logger)(&format!(
                                "skip exploration phase: cannot create GSOM network: {err}"
                            ));
                            self.phase = RosomaxaPhases::Exploitation { selection_size };
                        }
                    }
                }
            }
            RosomaxaPhases::Exploration {
                network,
                new_input_count,
                is_network_warmed_up,
                selection_coordinates,
                local_optimum_candidates,
                statistics: old_statistics,
                selection_size: old_selection_size,
            } => {
                if statistics.termination_estimate < exploration_ratio {
                    *old_statistics = statistics.clone();
                    *old_selection_size = selection_size;

                    Self::optimize_network(
                        &self.external_ctx,
                        network,
                        new_input_count,
                        is_network_warmed_up,
                        statistics,
                        &self.config,
                    );

                    Self::prepare_selection(
                        network,
                        selection_coordinates,
                        local_optimum_candidates,
                        self.environment.random.as_ref(),
                        self.objective.as_ref(),
                        statistics.generation,
                        selection_size,
                    );
                } else {
                    self.phase = RosomaxaPhases::Exploitation { selection_size }
                }
            }
            RosomaxaPhases::Exploitation { selection_size: old_selection_size, .. } => {
                // NOTE as we exploit elite only, limit how many solutions are exploited simultaneously
                *old_selection_size = get_exploitation_selection_size(*old_selection_size)
            }
        }
    }

    fn is_comparable_with_best_known(&self, individual: &S, best_known: Option<&S>) -> bool {
        best_known.is_none_or(|best_known| self.objective.total_order(individual, best_known) != Ordering::Greater)
    }

    fn optimize_network(
        external_ctx: &C,
        network: &mut IndividualNetwork<C, O, S>,
        new_input_count: &mut usize,
        is_network_warmed_up: &mut bool,
        statistics: &HeuristicStatistics,
        config: &RosomaxaConfig,
    ) {
        network.set_learning_rate(get_learning_rate(statistics.termination_estimate));

        // Check distortion after each node has seen about one new solution on average. A small floor avoids repeatedly
        // rebuilding a young map from just a few inputs.
        let observation_count = network.size().max(config.max_network_size.div_ceil(6));
        let observation_count = if *is_network_warmed_up {
            observation_count
        } else {
            observation_count.saturating_mul(INITIAL_GROWTH_OBSERVATION_WINDOWS)
        };
        if *new_input_count >= observation_count {
            // The map has seen enough inputs to use the regular cadence even when it does not need smoothing yet.
            *is_network_warmed_up = true;

            // set the MSE threshold to a fraction of the maximum possible normalized distance
            let mse = network.mse();
            let threshold = 0.5 / (network.dimension() as Float).sqrt();
            if mse > threshold {
                network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
            }
            *new_input_count = 0;
        }

        let keep_size = get_keep_size(config.max_network_size, statistics.termination_estimate);
        // no need to shrink network
        if network.size() <= keep_size {
            return;
        }

        network.compact(external_ctx);
        network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
        *is_network_warmed_up = true;
    }

    fn prepare_selection(
        network: &IndividualNetwork<C, O, S>,
        selection_coordinates: &mut Vec<Coordinate>,
        local_optimum_candidates: &mut Vec<(usize, Float)>,
        random: &dyn Random,
        objective: &O,
        generation: usize,
        selection_size: usize,
    ) {
        selection_coordinates.clear();
        selection_coordinates.extend(network.iter().filter_map(|(coordinate, node)| {
            if node.storage.population.size() > 0 { Some(*coordinate) } else { None }
        }));

        selection_coordinates.shuffle(&mut random.get_rng());

        // Give a local optimum another chance to occupy the first GSOM slot; keep the rest shuffled.
        if selection_size > 2 && selection_coordinates.len() > 1 {
            Self::promote_coordinate(selection_coordinates, random, |coordinate| {
                Self::is_local_optimum(network, coordinate, objective)
            });

            // Periodically reserve the second GSOM slot for a smooth local optimum far from the first.
            if selection_size > 6 && generation.is_multiple_of(4) {
                Self::promote_diverse_local_optimum(
                    network,
                    selection_coordinates,
                    local_optimum_candidates,
                    objective,
                );
            }
        }
    }

    fn promote_coordinate(
        coordinates: &mut [Coordinate],
        random: &dyn Random,
        is_preferred: impl Fn(&Coordinate) -> bool,
    ) {
        debug_assert!(coordinates.len() > 1);
        let candidate_idx = random.uniform_int(1, coordinates.len() as i32 - 1) as usize;
        if !is_preferred(&coordinates[0]) && is_preferred(&coordinates[candidate_idx]) {
            coordinates.swap(0, candidate_idx);
        }
    }

    fn promote_diverse_local_optimum(
        network: &IndividualNetwork<C, O, S>,
        coordinates: &mut [Coordinate],
        local_optima: &mut Vec<(usize, Float)>,
        objective: &O,
    ) {
        local_optima.clear();

        let reference_coordinate = coordinates[0];
        let Some(reference_node) = network.find(&reference_coordinate) else { return };
        let reference_weights = reference_node.weights.as_slice();

        // A shuffled reference rotates which distant basin is promoted.
        local_optima.extend(coordinates.iter().copied().enumerate().skip(1).filter_map(|(index, coordinate)| {
            let node = network.find(&coordinate)?;
            let solution = node.storage.population.best()?;

            Self::is_local_optimum_solution(network, &coordinate, solution, objective).then(|| {
                let smoothness = node.unified_distance(network, 1);
                (index, smoothness)
            })
        }));

        let Some(coordinate_idx) = Self::select_diverse_local_optimum(local_optima, |index| {
            network
                .find(&coordinates[index])
                .map(|node| network.distance(node.weights.as_slice(), reference_weights))
                .unwrap_or_default()
        }) else {
            return;
        };

        coordinates.swap(1, coordinate_idx);
    }

    fn select_diverse_local_optimum(
        candidates: &mut [(usize, Float)],
        distance: impl Fn(usize) -> Float,
    ) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }

        // Smoothness filters map noise; distance provides a distinct search direction.
        let middle = candidates.len() / 2;
        candidates.select_nth_unstable_by(middle, |(_, left), (_, right)| left.total_cmp(right));
        candidates[..=middle]
            .iter()
            .map(|(index, _)| (*index, distance(*index)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
    }

    fn is_local_optimum(network: &IndividualNetwork<C, O, S>, coordinate: &Coordinate, objective: &O) -> bool {
        let Some(node) = network.find(coordinate) else { return false };
        let Some(solution) = node.storage.population.best() else { return false };

        Self::is_local_optimum_solution(network, coordinate, solution, objective)
    }

    fn is_local_optimum_solution(
        network: &IndividualNetwork<C, O, S>,
        coordinate: &Coordinate,
        solution: &S,
        objective: &O,
    ) -> bool {
        let Coordinate(x, y) = *coordinate;

        let comparisons = [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
            .into_iter()
            .filter_map(|coordinate| network.find(&coordinate))
            .filter_map(|node| node.storage.population.best())
            .map(|neighbor| objective.total_order(solution, neighbor));

        Self::is_strict_local_optimum(comparisons)
    }

    fn is_strict_local_optimum(comparisons: impl Iterator<Item = Ordering>) -> bool {
        let mut has_worse_neighbor = false;
        let mut has_neighbor = false;

        for comparison in comparisons {
            has_neighbor = true;
            match comparison {
                Ordering::Greater => return false,
                Ordering::Less => has_worse_neighbor = true,
                Ordering::Equal => {}
            }
        }

        has_neighbor && has_worse_neighbor
    }

    fn create_network(
        context: &C,
        objective: Arc<O>,
        environment: Arc<Environment>,
        config: &RosomaxaConfig,
        individuals: Vec<S>,
    ) -> GenericResult<IndividualNetwork<C, O, S>> {
        let inputs_vec = parallel_into_collect(individuals, ParallelismScope::Local, |i| init_individual(context, i));
        let inputs_vec = Self::select_initial_data(inputs_vec, objective.as_ref(), INITIAL_NETWORK_SIZE);

        Network::new(
            context,
            inputs_vec,
            NetworkConfig {
                node_size: config.node_size,
                spread_factor: config.spread_factor,
                distribution_factor: config.distribution_factor,
                learning_rate: 0.3,
                hit_memory_size: HIT_MEMORY_SIZE,
                has_initial_error: true,
            },
            environment.random.clone(),
            {
                let objective = objective.clone();
                let random = environment.random.clone();
                move |node_size| IndividualStorageFactory {
                    node_size,
                    random: random.clone(),
                    objective: objective.clone(),
                }
            },
        )
    }

    fn select_initial_data(mut data: Vec<S>, objective: &O, keep_size: usize) -> Vec<S> {
        if data.len() <= keep_size {
            return data;
        }

        data.sort_by(|left, right| objective.total_order(left, right));

        // Remove the weakest quarter before looking at feature distance. Otherwise, an incomplete or otherwise poor
        // solution can be retained simply because it is far away from every useful solution.
        let candidate_size = data.len().saturating_mul(3).div_ceil(4).max(keep_size).min(data.len());
        data.truncate(candidate_size);

        let mut selected = Vec::with_capacity(keep_size);
        selected.push(data.swap_remove(0));

        while selected.len() < keep_size && !data.is_empty() {
            let candidate_idx = data
                .iter()
                .map(|candidate| {
                    selected
                        .iter()
                        .map(|known| relative_distance(candidate.weights().iter(), known.weights().iter()))
                        .min_by(Float::total_cmp)
                        .expect("at least one initial solution is selected")
                })
                .enumerate()
                .max_by(|(_, left), (_, right)| left.total_cmp(right))
                .map(|(index, _)| index)
                .expect("at least one initial solution remains");

            selected.push(data.swap_remove(candidate_idx));
        }

        selected
    }
}

impl<'a, C, O, S> TryFrom<&'a Rosomaxa<C, O, S>> for NetworkState
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
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
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    Initial {
        solutions: Vec<S>,
    },
    Exploration {
        network: IndividualNetwork<C, O, S>,
        // Inputs added since the last distortion check; smoothing and compaction replay does not contribute.
        new_input_count: usize,
        is_network_warmed_up: bool,
        selection_coordinates: Vec<Coordinate>,
        // Reuse this scratch space instead of allocating while preparing each selection.
        local_optimum_candidates: Vec<(usize, Float)>,
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
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    node_size: usize,
    random: Arc<dyn Random>,
    objective: Arc<O>,
}

impl<C, O, S> StorageFactory<C, S, IndividualStorage<C, O, S>> for IndividualStorageFactory<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
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

        elitism.maybe_change();

        IndividualStorage { population: elitism }
    }
}

struct IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    population: Elitism<O, S>,
}

impl<C, O, S> IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    fn select(&self, random: &dyn Random, alternative_probability: Float) -> Option<&S> {
        let rank = match self.population.size() {
            0 => return None,
            size if size > 1 && random.is_hit(alternative_probability) => {
                random.uniform_int(1, size as i32 - 1) as usize
            }
            _ => 0,
        };

        self.population.get(rank)
    }
}

impl<C, O, S> Storage for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    type Item = S;

    fn add(&mut self, input: Self::Item) {
        self.population.add(input);
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Item> + '_> {
        Box::new(self.population.ranked())
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        self.population.drain(range).into_iter().collect()
    }

    fn resize(&mut self, size: usize) {
        self.population.set_max_population_size(size);
    }

    fn size(&self) -> usize {
        self.population.size()
    }
}

impl<C, O, S> Display for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.population)
    }
}

fn create_dedup_fn<C, O, S>(threshold: Float) -> DedupFn<O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
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
            let distance = relative_distance(weights_a.iter(), weights_b.iter());

            distance < threshold
        }
    })
}

/// Gets the elite share of the exploration budget. Each block of four parents adds one elite selection tournament,
/// with more elite searches used when the population is not improving.
fn get_elite_selection_size(
    selection_size: usize,
    improvement_ratio: Float,
    mut is_hit: impl FnMut(Float) -> bool,
) -> usize {
    if selection_size <= 6 {
        return 1;
    }

    let probability = (1. - 1. / (1. + E.powf(-10. * (improvement_ratio - 0.166)))) as Float;
    let tournament_count = selection_size.div_ceil(4);

    (1..=tournament_count)
        .map(|idx| if is_hit(probability / idx as Float) { 2 } else { 1 })
        .sum::<usize>()
        .min(selection_size - 1)
}

/// Cools direct node-alternative sampling as the map matures. Retained alternatives still participate in replay.
fn get_node_alternative_probability(termination_estimate: Float) -> Float {
    const INITIAL_PROBABILITY: Float = 0.05;

    INITIAL_PROBABILITY * (1. - termination_estimate.clamp(0., 1.))
}

/// Gets the exploitation budget by using half of the configured selection capacity.
fn get_exploitation_selection_size(selection_size: usize) -> usize {
    selection_size.div_ceil(2).max(2)
}

/// Gets network size to keep.
/// Slowly decreases the network limit from its configured maximum to one third of it.
fn get_keep_size(max_network_size: usize, termination_estimate: Float) -> usize {
    #![allow(clippy::unnecessary_cast)]
    let termination_estimate = termination_estimate.clamp(0., 0.8) as f64;
    // Sigmoid: https://www.wolframalpha.com/input?i=plot+1+*+%281%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let rate = 1. / (1. + E.powf(-10. * (termination_estimate - 0.5)));
    let min_network_size = (max_network_size / 3).max(4);
    let network_size_range = max_network_size - min_network_size;

    min_network_size + (network_size_range as Float * (1. - rate) as Float) as usize
}

/// Keeps exploration active a bit longer when it is still improving solutions.
fn get_exploration_ratio(exploration_ratio: Float, improvement_ratio: Float) -> Float {
    const EXPLORATION_EXTENSION: Float = 0.05;
    const MAX_EXPLORATION_RATIO: Float = 0.95;

    if exploration_ratio == 0. || improvement_ratio <= 0. {
        exploration_ratio
    } else {
        // Keep a final exploitation window and do not shorten an explicitly larger ratio.
        (exploration_ratio + EXPLORATION_EXTENSION).min(MAX_EXPLORATION_RATIO).max(exploration_ratio)
    }
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
