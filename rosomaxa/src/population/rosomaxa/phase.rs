use super::maintenance::{NetworkMaintenance, optimize_network};
use super::relaxed::prune_relaxed_archive;
use super::selection::{BasinCandidate, get_exploitation_selection_size, prepare_selection, select_initial_data};
use super::storage::IndividualStorageFactory;
use super::{IndividualNetwork, Rosomaxa, RosomaxaConfig, RosomaxaContext, RosomaxaSolution};
use crate::algorithms::gsom::{Coordinate, Network, NetworkConfig, NetworkState, get_network_state};
use crate::evolution::objectives::HeuristicObjective;
use crate::population::elitism::Alternative;
use crate::utils::{Environment, Float, GenericResult, ParallelismPolicy, parallel_collect};
use crate::{HeuristicSpeed, HeuristicStatistics};
use std::sync::Arc;

// Hit history is exposed in GSOM state, but does not control its maintenance or capacity.
const HIT_MEMORY_SIZE: usize = 200;

// A larger candidate pool gives different constructors a chance to improve before GSOM is trained. Keep its input
// bounded to the previous default size so weak outliers do not increase training work or shape the whole map.
const INITIAL_NETWORK_SIZE: usize = 16;

#[allow(clippy::large_enum_variant)]
pub(super) enum RosomaxaPhases<C, O, S>
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
        relaxed: Vec<S>,
    },
}

impl<C, O, S> Rosomaxa<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    pub(super) fn update_phase(&mut self, statistics: &HeuristicStatistics) {
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
                    self.phase = RosomaxaPhases::Exploitation { selection_size, relaxed: Vec::new() }
                } else if individuals.len() >= self.config.initial_size {
                    let network_result = create_network(
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
                            self.phase = RosomaxaPhases::Exploitation { selection_size, relaxed: Vec::new() };
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

                    optimize_network(&self.external_ctx, network, maintenance, statistics, &self.config);

                    prepare_selection(
                        network,
                        selection_coordinates,
                        basin_candidates,
                        self.environment.random.as_ref(),
                        self.objective.as_ref(),
                        statistics,
                        selection_size,
                    );
                } else {
                    let mut relaxed =
                        network.iter_nodes_mut().flat_map(|node| node.storage.relaxed.drain_all()).collect();
                    prune_relaxed_archive(&mut relaxed, self.objective.as_ref(), self.config.selection_size);
                    self.phase = RosomaxaPhases::Exploitation { selection_size, relaxed }
                }
            }
            RosomaxaPhases::Exploitation { selection_size: old_selection_size, .. } => {
                // NOTE as we exploit elite only, limit how many solutions are exploited simultaneously
                *old_selection_size = get_exploitation_selection_size(*old_selection_size);
            }
        }
    }
}

fn create_network<C, O, S>(
    context: &C,
    objective: Arc<O>,
    environment: Arc<Environment>,
    config: &RosomaxaConfig,
    individuals: Vec<S>,
) -> GenericResult<IndividualNetwork<C, O, S>>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    let inputs_vec = parallel_collect(individuals, ParallelismPolicy::Default, |i| init_individual(context, i));
    let inputs_vec = select_initial_data(inputs_vec, objective.as_ref(), INITIAL_NETWORK_SIZE);

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

pub(super) fn init_individual<C, S>(external_ctx: &C, individual: S) -> S
where
    C: RosomaxaContext<Solution = S>,
    S: RosomaxaSolution<Context = C>,
{
    let mut individual = individual;
    individual.on_init(external_ctx);

    individual
}

pub(super) fn get_exploration_ratio(exploration_ratio: Float, improvement_ratio: Float) -> Float {
    const EXPLORATION_EXTENSION: Float = 0.05;
    const MAX_EXPLORATION_RATIO: Float = 0.95;

    if exploration_ratio == 0. || improvement_ratio <= 0. {
        exploration_ratio
    } else {
        // Keep a final exploitation window and do not shorten an explicitly larger ratio.
        (exploration_ratio + EXPLORATION_EXTENSION).min(MAX_EXPLORATION_RATIO).max(exploration_ratio)
    }
}
