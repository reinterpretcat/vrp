#[cfg(test)]
#[path = "../../../tests/unit/population/rosomaxa/mod.rs"]
mod rosomaxa_test;

mod maintenance;
mod phase;
mod relaxed;
mod selection;
mod storage;

use self::phase::{RosomaxaPhases, init_individual};
use self::relaxed::{prune_relaxed, prune_relaxed_archive, select_relaxed_archive, select_relaxed_solution};
use self::selection::{get_elite_selection_size, get_node_alternative_probability};
use self::storage::{IndividualStorage, IndividualStorageFactory, create_dedup_fn};
use super::{HeuristicPopulation, SelectionPhase};
use crate::algorithms::gsom::{Input, Network};
use crate::evolution::objectives::HeuristicObjective;
use crate::population::elitism::{Alternative, Elitism};
use crate::utils::{Environment, ParallelismPolicy, parallel_collect};
use crate::utils::{Float, GenericError};
use crate::{HeuristicSolution, HeuristicStatistics};
use std::cmp::Ordering;
use std::sync::Arc;

/// Specifies rosomaxa configuration settings.
pub struct RosomaxaConfig {
    /// Number of candidates collected before GSOM initialization.
    pub initial_size: usize,
    /// Number of parents selected per generation. During relaxed admission it also bounds how many GSOM regions keep
    /// hidden lineages; each retained region can store a boundary and a bridge representative.
    pub selection_size: usize,
    /// Elite population size.
    pub elite_size: usize,
    /// Maximum number of regular solutions retained by each GSOM node. Hidden relaxed boundary and bridge
    /// representatives are bounded separately. Smoothing replays every resident, while regular parent selection
    /// normally takes one solution per node and favors its best one.
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

    /// Returns the normalized violation of a solution created under controlled relaxed constraints.
    fn relaxed_violation(&self) -> Option<Float> {
        None
    }
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
            .filter(|individual| individual.relaxed_violation().is_none())
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
                let data = parallel_collect(individuals, ParallelismPolicy::Default, |i| {
                    init_individual(&self.external_ctx, i)
                });
                let has_relaxed = data.iter().any(|individual| individual.relaxed_violation().is_some());
                maintenance.add_observations(data.len());
                network.store_batch(&self.external_ctx, data, statistics.generation);
                if has_relaxed {
                    prune_relaxed(network, self.objective.as_ref(), self.config.selection_size);
                }
            }
            RosomaxaPhases::Exploitation { relaxed, .. } => {
                let incoming = individuals
                    .into_iter()
                    .filter(|individual| individual.relaxed_violation().is_some())
                    .map(|individual| init_individual(&self.external_ctx, individual))
                    .collect::<Vec<_>>();
                if !incoming.is_empty() {
                    relaxed.extend(incoming);
                    prune_relaxed_archive(relaxed, self.objective.as_ref(), self.config.selection_size);
                }
            }
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

    fn select_relaxed(&self, reference: &Self::Individual) -> Option<&Self::Individual> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                select_relaxed_solution(network, self.objective.as_ref(), reference, self.config.selection_size)
            }
            RosomaxaPhases::Exploitation { relaxed, .. } => {
                select_relaxed_archive(relaxed, self.objective.as_ref(), reference)
            }
            RosomaxaPhases::Initial { .. } => None,
        }
    }

    fn supports_relaxed_search(&self) -> bool {
        true
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        match &self.phase {
            RosomaxaPhases::Exploration { network, .. } => {
                Box::new(self.elite.iter().chain(network.iter_nodes().flat_map(|node| node.storage.regular.iter())))
            }
            _ => self.elite.iter(),
        }
    }

    fn into_iter(self: Box<Self>) -> Box<dyn Iterator<Item = Self::Individual>> {
        match self.phase {
            RosomaxaPhases::Exploration { network, .. } => Box::new(
                Box::new(self.elite)
                    .into_iter()
                    .chain(network.into_iter_nodes().flat_map(|(_, node)| Box::new(node.storage.regular).into_iter())),
            ),
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
        let is_valid_factor = |factor: Float| factor > 0. && factor < 1.;
        if !is_valid_factor(config.spread_factor) || !is_valid_factor(config.distribution_factor) {
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

    fn is_comparable_with_best_known(&self, individual: &S, best_known: Option<&S>) -> bool {
        best_known.is_none_or(|best_known| self.objective.total_order(individual, best_known) != Ordering::Greater)
    }
}
