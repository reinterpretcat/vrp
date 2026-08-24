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
            RosomaxaPhases::Exploration { network, maintenance, statistics, .. } => {
                self.external_ctx.on_change(individuals.as_slice());
                let data = parallel_into_collect(individuals, ParallelismScope::Local, |i| {
                    init_individual(&self.external_ctx, i)
                });
                maintenance.add_observations(data.len());
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

// A larger candidate pool gives different constructors a chance to improve before GSOM is trained. Keep its input
// bounded to the previous default size so weak outliers do not increase training work or shape the whole map.
const INITIAL_NETWORK_SIZE: usize = 16;

// Three generations revisit promising GSOM regions, while the fourth samples the whole occupied map uniformly.
const BASIN_SELECTION_PERIOD: usize = 4;

// Reserve roughly one quarter of the GSOM parent budget for uniformly sampled occupied nodes.
const BASIN_COVERAGE_DIVISOR: usize = 4;

/// Keeps a GSOM coordinate and its cached distance to the closest selected basin.
struct BasinCandidate {
    coordinate: Coordinate,
    min_distance: Float,
}

impl BasinCandidate {
    /// Creates a candidate whose nearest-selected distance will be initialized before ranking.
    fn new(coordinate: Coordinate) -> Self {
        Self { coordinate, min_distance: Float::MAX }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum NetworkMaintenanceAction {
    RefreshNormalization,
    CheckDistortion,
}

/// Keeps smoothing responsive without letting repeated full-map replay dominate ordinary training.
struct NetworkMaintenance {
    /// Inputs added since the last distortion check; smoothing and compaction replay does not contribute.
    new_input_count: usize,
    /// Inputs added since the normalization ranges were rebuilt from retained solutions.
    normalization_input_count: usize,
    /// Small maps wait for this many observations before checking distortion.
    min_observation_count: usize,
    /// Consecutive smoothing grows the evidence window; a stable observation gradually shrinks it again.
    observation_multiplier: usize,
    max_observation_multiplier: usize,
}

impl NetworkMaintenance {
    fn new(config: &RosomaxaConfig) -> Self {
        Self {
            new_input_count: 0,
            normalization_input_count: 0,
            min_observation_count: config.max_network_size.div_ceil(6),
            observation_multiplier: 1,
            max_observation_multiplier: get_max_smoothing_observation_multiplier(config.node_size),
        }
    }

    fn add_observations(&mut self, count: usize) {
        self.new_input_count = self.new_input_count.saturating_add(count);
        self.normalization_input_count = self.normalization_input_count.saturating_add(count);
    }

    fn base_observation_count(&self, network_size: usize) -> usize {
        network_size.max(self.min_observation_count)
    }

    fn next_action(&mut self, network_size: usize) -> Option<NetworkMaintenanceAction> {
        let observation_count = self.base_observation_count(network_size);
        let is_distortion_due = self.new_input_count >= observation_count.saturating_mul(self.observation_multiplier);
        let is_normalization_due = self.normalization_input_count >= observation_count;

        if is_distortion_due || is_normalization_due {
            self.normalization_input_count = 0;
        }

        if is_distortion_due {
            Some(NetworkMaintenanceAction::CheckDistortion)
        } else if is_normalization_due {
            Some(NetworkMaintenanceAction::RefreshNormalization)
        } else {
            None
        }
    }

    fn on_smoothing(&mut self) {
        self.new_input_count = 0;
        self.normalization_input_count = 0;
        self.observation_multiplier =
            self.observation_multiplier.saturating_mul(2).min(self.max_observation_multiplier);
    }

    fn on_stable_observation(&mut self) {
        self.new_input_count = 0;
        self.normalization_input_count = 0;
        self.observation_multiplier = self.observation_multiplier.div_ceil(2).max(1);
    }
}

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
                            let selection_coordinates = network.get_coordinates().collect::<Vec<_>>();
                            let basin_candidates = Vec::with_capacity(selection_coordinates.len());

                            self.phase = RosomaxaPhases::Exploration {
                                network,
                                maintenance: NetworkMaintenance::new(&self.config),
                                selection_coordinates,
                                basin_candidates,
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
                maintenance,
                selection_coordinates,
                basin_candidates,
                statistics: old_statistics,
                selection_size: old_selection_size,
            } => {
                if statistics.termination_estimate < exploration_ratio {
                    *old_statistics = statistics.clone();
                    *old_selection_size = selection_size;

                    Self::optimize_network(&self.external_ctx, network, maintenance, statistics, &self.config);

                    Self::prepare_selection(
                        network,
                        selection_coordinates,
                        basin_candidates,
                        self.environment.random.as_ref(),
                        self.objective.as_ref(),
                        statistics.generation,
                        statistics.improvement_1000_ratio,
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
        maintenance: &mut NetworkMaintenance,
        statistics: &HeuristicStatistics,
        config: &RosomaxaConfig,
    ) {
        network.set_learning_rate(get_learning_rate(statistics.termination_estimate));

        let keep_size = get_keep_size(config.max_network_size, statistics.termination_estimate);
        if network.size() > keep_size {
            // Compaction already rebuilds the map, so handle it before periodic smoothing and start a fresh
            // evidence window.
            network.compact(external_ctx);
            network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
            maintenance.on_smoothing();
            return;
        }

        match maintenance.next_action(network.size()) {
            Some(NetworkMaintenanceAction::RefreshNormalization) => {
                // Keep feature ranges representative of retained solutions even while expensive replay is backed off.
                network.refresh_normalization();
            }
            Some(NetworkMaintenanceAction::CheckDistortion) => {
                // Distortion has to be measured using the current retained population, not historical outliers.
                network.refresh_normalization();

                // Let a young map learn enough topology before smoothing can reset the errors which drive GSOM growth.
                let can_smooth = network.size() >= get_min_network_size(config.max_network_size);
                // Set the MSE threshold to a fraction of the maximum possible normalized distance.
                let threshold = 0.5 / (network.dimension() as Float).sqrt();
                let should_smooth = can_smooth && network.mse() > threshold;

                if should_smooth {
                    network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
                    maintenance.on_smoothing();
                } else {
                    maintenance.on_stable_observation();
                }
            }
            None => {}
        }
    }

    fn prepare_selection(
        network: &IndividualNetwork<C, O, S>,
        selection_coordinates: &mut Vec<Coordinate>,
        basin_candidates: &mut Vec<BasinCandidate>,
        random: &dyn Random,
        objective: &O,
        generation: usize,
        improvement_ratio: Float,
        selection_size: usize,
    ) {
        selection_coordinates.clear();
        selection_coordinates.extend(network.iter().filter_map(|(coordinate, node)| {
            if node.storage.population.size() > 0 { Some(*coordinate) } else { None }
        }));

        selection_coordinates.shuffle(&mut random.get_rng());

        // Periodic full-map batches keep exploration coherent instead of diluting every basin-focused batch.
        if !is_basin_selection_generation(generation) {
            return;
        }

        let node_selection_budget = selection_size.saturating_sub(get_min_elite_selection_size(selection_size));
        // Most node searches revisit distinct basins; a smaller share keeps every occupied region reachable.
        let coverage_selection_size = get_coverage_selection_size(node_selection_budget);
        let basin_selection_size = node_selection_budget.saturating_sub(coverage_selection_size);
        if basin_selection_size == 0 {
            return;
        }

        basin_candidates.clear();
        // Local sinks are cheap basin proxies: tracing every occupied node to its sink would add more map scans here.
        basin_candidates.extend(
            selection_coordinates
                .iter()
                .filter(|coordinate| Self::is_basin_sink_node(network, coordinate, objective))
                .map(|coordinate| BasinCandidate::new(*coordinate)),
        );
        let compare_quality = |left: &BasinCandidate, right: &BasinCandidate| {
            let left = network
                .find(&left.coordinate)
                .and_then(|node| node.storage.population.best())
                .expect("basin candidates are occupied GSOM nodes");
            let right = network
                .find(&right.coordinate)
                .and_then(|node| node.storage.population.best())
                .expect("basin candidates are occupied GSOM nodes");

            objective.total_order(left, right)
        };

        // Keep a rank-based quality gate independent of the objective's numeric scale. Small basin sets remain intact.
        let candidate_size = get_basin_candidate_size(basin_candidates.len(), basin_selection_size);
        if candidate_size < basin_candidates.len() {
            basin_candidates.select_nth_unstable_by(candidate_size, compare_quality);
            basin_candidates.truncate(candidate_size);
        }

        let selected_size = basin_selection_size.min(basin_candidates.len());
        if selected_size == 0 {
            return;
        }

        // Rotate the reference within the quality-gated set, then add candidates far from everything already selected.
        let reference_idx = random.uniform_int(0, basin_candidates.len() as i32 - 1) as usize;
        select_diverse_basin_candidates(basin_candidates, selected_size, reference_idx, |left, right| {
            let left = network.find(left).expect("basin candidates belong to the GSOM");
            let right = network.find(right).expect("basin candidates belong to the GSOM");
            network.squared_distance(left.weights.as_slice(), right.weights.as_slice())
        });

        // Occasionally replace the least distinctive basin with a good distant slope. Put it first so extra elite
        // selections cannot truncate the escape attempt from the GSOM prefix.
        if is_basin_shoulder_selection_generation(generation, improvement_ratio)
            && let Some(shoulder) =
                Self::select_basin_shoulder(network, selection_coordinates, basin_candidates, selected_size, objective)
        {
            basin_candidates[..selected_size].rotate_right(1);
            basin_candidates[0] = BasinCandidate::new(shoulder);
        }

        // Interleave basin representatives with shuffled nodes, preserving a direct path to every occupied region.
        promote_basin_coordinates(
            selection_coordinates,
            &basin_candidates[..selected_size],
            node_selection_budget,
            coverage_selection_size,
        );
    }

    /// Checks whether the best solution in a GSOM node represents a map-local basin sink.
    fn is_basin_sink_node(network: &IndividualNetwork<C, O, S>, coordinate: &Coordinate, objective: &O) -> bool {
        let Some(node) = network.find(coordinate) else { return false };
        let Some(solution) = node.storage.population.best() else { return false };
        let Coordinate(x, y) = *coordinate;

        let neighbors = [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
            .into_iter()
            .filter_map(|neighbor_coordinate| {
                let neighbor = network.find(&neighbor_coordinate)?;
                let neighbor_solution = neighbor.storage.population.best()?;
                Some((neighbor_coordinate, objective.total_order(solution, neighbor_solution)))
            });

        is_basin_sink(coordinate, neighbors)
    }

    /// Selects a good non-minimum node far from the basin representatives as a cheap approximation of a basin shoulder.
    fn select_basin_shoulder(
        network: &IndividualNetwork<C, O, S>,
        occupied: &[Coordinate],
        basin_candidates: &[BasinCandidate],
        selected_size: usize,
        objective: &O,
    ) -> Option<Coordinate> {
        if selected_size < 2 {
            return None;
        }

        let mut candidates = occupied
            .iter()
            .filter(|coordinate| !Self::is_basin_sink_node(network, coordinate, objective))
            .copied()
            .collect::<Vec<_>>();
        let get_node =
            |coordinate: &Coordinate| network.find(coordinate).expect("selection candidates belong to the GSOM");
        let get_solution = |coordinate: &Coordinate| {
            get_node(coordinate).storage.population.best().expect("selection candidates have a solution")
        };
        let compare =
            |left: &Coordinate, right: &Coordinate| objective.total_order(get_solution(left), get_solution(right));

        let candidate_size = get_basin_candidate_size(candidates.len(), 1);
        if candidate_size < candidates.len() {
            candidates.select_nth_unstable_by(candidate_size, compare);
            candidates.truncate(candidate_size);
        }

        let selected_weights = basin_candidates[..selected_size]
            .iter()
            .map(|candidate| get_node(&candidate.coordinate).weights.as_slice())
            .collect::<Vec<_>>();
        candidates.into_iter().max_by(|left, right| {
            let min_distance = |coordinate: &Coordinate| {
                let weights = get_node(coordinate).weights.as_slice();
                selected_weights
                    .iter()
                    .map(|selected| network.squared_distance(weights, selected))
                    .min_by(Float::total_cmp)
                    .unwrap_or_default()
            };

            min_distance(left).total_cmp(&min_distance(right)).then_with(|| compare(right, left))
        })
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
        maintenance: NetworkMaintenance,
        // Occupied nodes in the order used by the next parent selection.
        selection_coordinates: Vec<Coordinate>,
        // Reuse this scratch space instead of allocating while preparing each selection.
        basin_candidates: Vec<BasinCandidate>,
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

/// Moves selected basin representatives into the parent prefix while preserving uniformly sampled coverage slots.
fn promote_basin_coordinates(
    coordinates: &mut [Coordinate],
    basins: &[BasinCandidate],
    selection_size: usize,
    coverage_size: usize,
) {
    let mut basin_idx = 0;
    let mut coverage_idx = 0;
    let selection_size = selection_size.min(coordinates.len());

    for selection_idx in 0..selection_size {
        // Place coverage at the midpoint of each equally sized part of the selected prefix.
        let is_coverage_slot = coverage_idx < coverage_size
            && selection_idx == (2 * coverage_idx + 1) * selection_size / (2 * coverage_size);

        if is_coverage_slot {
            let remaining_basins = &basins[basin_idx..];
            if remaining_basins.iter().any(|candidate| candidate.coordinate == coordinates[selection_idx])
                && let Some(candidate_idx) = ((selection_idx + 1)..coordinates.len()).find(|&candidate_idx| {
                    remaining_basins.iter().all(|candidate| candidate.coordinate != coordinates[candidate_idx])
                })
            {
                coordinates.swap(selection_idx, candidate_idx);
            }
            coverage_idx += 1;
        } else if let Some(candidate) = basins.get(basin_idx)
            && let Some(candidate_idx) =
                (selection_idx..coordinates.len()).find(|&idx| coordinates[idx] == candidate.coordinate)
        {
            coordinates.swap(selection_idx, candidate_idx);
            basin_idx += 1;
        }
    }
}

/// Orders the selected prefix using incremental maximum--minimum distance from the already selected representatives.
fn select_diverse_basin_candidates(
    candidates: &mut [BasinCandidate],
    selected_size: usize,
    reference_idx: usize,
    distance: impl Fn(&Coordinate, &Coordinate) -> Float,
) {
    let selected_size = selected_size.min(candidates.len());
    if selected_size == 0 {
        return;
    }

    candidates.swap(0, reference_idx);
    if selected_size == 1 {
        return;
    }

    let reference = candidates[0].coordinate;
    for candidate in &mut candidates[1..] {
        candidate.min_distance = distance(&candidate.coordinate, &reference);
    }

    for selected_idx in 1..selected_size {
        let candidate_idx = (selected_idx..candidates.len())
            .max_by(|&left, &right| candidates[left].min_distance.total_cmp(&candidates[right].min_distance))
            .expect("at least one basin candidate remains");

        candidates.swap(selected_idx, candidate_idx);

        if selected_idx + 1 < selected_size {
            let selected = candidates[selected_idx].coordinate;
            for candidate in &mut candidates[(selected_idx + 1)..] {
                let candidate_distance = distance(&candidate.coordinate, &selected);
                if candidate_distance.total_cmp(&candidate.min_distance).is_lt() {
                    candidate.min_distance = candidate_distance;
                }
            }
        }
    }
}

/// Returns true when no cardinal neighbor dominates the coordinate, resolving equal-fitness plateaus by coordinate.
fn is_basin_sink(coordinate: &Coordinate, mut neighbors: impl Iterator<Item = (Coordinate, Ordering)>) -> bool {
    // Coordinate order gives equal-fitness plateaus a stable direction instead of selecting every plateau node.
    neighbors.all(|(neighbor, order)| order == Ordering::Less || (order == Ordering::Equal && *coordinate < neighbor))
}

/// Gets the elite share of the exploration budget. Each block of four parents adds one elite selection tournament,
/// with more elite searches used when the population is not improving.
fn get_elite_selection_size(
    selection_size: usize,
    improvement_ratio: Float,
    mut is_hit: impl FnMut(Float) -> bool,
) -> usize {
    if selection_size <= 6 {
        return get_min_elite_selection_size(selection_size);
    }

    let probability = (1. - 1. / (1. + E.powf(-10. * (improvement_ratio - 0.166)))) as Float;
    let tournament_count = selection_size.div_ceil(4);

    (1..=tournament_count)
        .map(|idx| if is_hit(probability / idx as Float) { 2 } else { 1 })
        .sum::<usize>()
        .min(selection_size - 1)
}

/// Returns the elite budget guaranteed before randomized extra elite tournaments are considered.
fn get_min_elite_selection_size(selection_size: usize) -> usize {
    if selection_size <= 6 { selection_size.min(1) } else { selection_size.div_ceil(4) }
}

/// Keeps the better half of basin sinks, or enough candidates to fill all basin positions.
fn get_basin_candidate_size(basin_count: usize, selection_size: usize) -> usize {
    basin_count.div_ceil(2).max(selection_size).min(basin_count)
}

/// Uses basin-focused ordering for three generations and uniform map ordering for the fourth.
fn is_basin_selection_generation(generation: usize) -> bool {
    !generation.is_multiple_of(BASIN_SELECTION_PERIOD)
}

/// Tests a basin shoulder occasionally while search is progressing and every basin cycle when it is nearly stagnant.
fn is_basin_shoulder_selection_generation(generation: usize, improvement_ratio: Float) -> bool {
    let period =
        if improvement_ratio <= 0.1 { BASIN_SELECTION_PERIOD } else { BASIN_SELECTION_PERIOD * BASIN_SELECTION_PERIOD };

    generation % period == 1
}

/// Reserves roughly one quarter of node positions for uniformly shuffled map coverage.
fn get_coverage_selection_size(selection_size: usize) -> usize {
    if selection_size <= 1 {
        return 0;
    }

    (selection_size / BASIN_COVERAGE_DIVISOR).max(1)
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

/// Gets the minimum useful network size derived from its configured capacity.
fn get_min_network_size(max_network_size: usize) -> usize {
    (max_network_size / 3).max(4).min(max_network_size)
}

/// Caps adaptive smoothing backoff. Four ordinary assignments per retained node item keep steady-state replay work
/// near one quarter of ordinary GSOM assignment work for the usual small node capacities. The upper bound keeps
/// maintenance reachable when a custom configuration uses larger node storage.
fn get_max_smoothing_observation_multiplier(node_size: usize) -> usize {
    node_size.saturating_mul(4).clamp(4, 16)
}

/// Gets the network size at which compaction is triggered.
/// Slowly decreases the trigger from the configured maximum to two thirds of it. As compaction retains roughly half
/// of the lattice, the map does not fall far below one third of its configured capacity.
fn get_keep_size(max_network_size: usize, termination_estimate: Float) -> usize {
    #![allow(clippy::unnecessary_cast)]
    let termination_estimate = termination_estimate.clamp(0., 0.8) as f64;
    // Sigmoid: https://www.wolframalpha.com/input?i=plot+1+*+%281%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let rate = 1. / (1. + E.powf(-10. * (termination_estimate - 0.5)));
    let min_compaction_size = get_min_network_size(max_network_size).saturating_mul(2).min(max_network_size);
    let network_size_range = max_network_size - min_compaction_size;

    min_compaction_size + (network_size_range as Float * (1. - rate) as Float) as usize
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
