#[cfg(test)]
#[path = "../../../tests/unit/solver/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use crate::algorithms::mdp::*;
use crate::algorithms::nsga2::{MultiObjective, Objective};
use crate::algorithms::statistics::relative_distance;
use crate::models::common::{MultiDimLoad, SingleDimLoad};
use crate::models::Problem;
use crate::solver::hyper::{HyperHeuristic, StaticSelective};
use crate::solver::mutation::*;
use crate::solver::population::Individual;
use crate::solver::RefinementContext;
use crate::utils::{compare_floats, Environment};
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

/// An experimental dynamic selective hyper heuristic which selects inner heuristics
/// based on how they work during the search. The selection process is modeled by
/// Markov Decision Process.
pub struct DynamicSelective {
    heuristic_simulator: Simulator<SearchState>,
    initial_estimates: HashMap<SearchState, ActionEstimates<SearchState>>,
    action_registry: SearchActionRegistry,
}

impl HyperHeuristic for DynamicSelective {
    fn search(&mut self, refinement_ctx: &RefinementContext, individuals: Vec<&Individual>) -> Vec<Individual> {
        let registry = &self.action_registry;
        let estimates = &self.initial_estimates;

        let agents = individuals
            .into_iter()
            .map(|individual| {
                Box::new(SearchAgent {
                    refinement_ctx,
                    original_ctx: individual,
                    registry,
                    estimates,
                    state: match compare_to_best(refinement_ctx, individual) {
                        Ordering::Greater => SearchState::Diverse,
                        _ => SearchState::BestKnown,
                    },
                    individual: Some(individual.deep_copy()),
                })
            })
            .collect();

        let individuals = self
            .heuristic_simulator
            .run_episodes(agents, refinement_ctx.environment.parallelism.clone(), |state, values| match state {
                SearchState::BestKnown => values.iter().max_by(|a, b| compare_floats(**a, **b)).cloned().unwrap_or(0.),
                _ => values.iter().sum::<f64>() / values.len() as f64,
            })
            .into_iter()
            .filter_map(|agent| agent.individual)
            .collect();

        try_exchange_estimates(&mut self.heuristic_simulator);

        individuals
    }
}

impl DynamicSelective {
    /// Creates a new instance of `DynamicSelective`.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        let mutations = Self::get_mutations(problem, environment.clone());
        let mutation_estimates = Self::get_estimates(mutations.as_slice());

        Self {
            heuristic_simulator: Simulator::new(
                Box::new(MonteCarlo::new(0.1)),
                Box::new(EpsilonWeighted::new(0.1, environment.random.clone())),
            ),
            initial_estimates: vec![
                (SearchState::BestKnown, mutation_estimates.clone()),
                (SearchState::Diverse, mutation_estimates),
                (SearchState::BestMajorImprovement, Default::default()),
                (SearchState::BestMinorImprovement, Default::default()),
                (SearchState::DiverseImprovement, Default::default()),
                (SearchState::Stagnated, Default::default()),
            ]
            .into_iter()
            .collect(),
            action_registry: SearchActionRegistry { mutations },
        }
    }

    fn get_mutations(
        problem: Arc<Problem>,
        environment: Arc<Environment>,
    ) -> Vec<(Arc<dyn Mutation + Send + Sync>, String)> {
        let random = environment.random.clone();
        let recreates: Vec<(Arc<dyn Recreate + Send + Sync>, String)> = vec![
            (Arc::new(RecreateWithSkipBest::new(1, 2, random.clone())), "skip_best_1".to_string()),
            (Arc::new(RecreateWithSkipBest::new(1, 4, random.clone())), "skip_best_2".to_string()),
            (Arc::new(RecreateWithRegret::new(1, 3, random.clone())), "regret".to_string()),
            (Arc::new(RecreateWithCheapest::new(random.clone())), "cheapest".to_string()),
            (Arc::new(RecreateWithPerturbation::new_with_defaults(random.clone())), "perturbation".to_string()),
            (Arc::new(RecreateWithGaps::new(2, 20, random.clone())), "gaps".to_string()),
            (
                Arc::new(RecreateWithBlinks::<SingleDimLoad>::new_with_defaults(random.clone())),
                "blinks_single".to_string(),
            ),
            (
                Arc::new(RecreateWithBlinks::<MultiDimLoad>::new_with_defaults(random.clone())),
                "blinks_multi".to_string(),
            ),
            (Arc::new(RecreateWithFarthest::new(random.clone())), "farthest".to_string()),
            (Arc::new(RecreateWithNearestNeighbor::new(random.clone())), "nearest".to_string()),
            (
                Arc::new(RecreateWithSkipRandom::default_explorative_phased(
                    Arc::new(RecreateWithCheapest::new(random.clone())),
                    random.clone(),
                )),
                "skip_random".to_string(),
            ),
            (Arc::new(RecreateWithSlice::new(random.clone())), "slice".to_string()),
        ];

        let primary_ruins: Vec<(Arc<dyn Ruin + Send + Sync>, String)> = vec![
            (Arc::new(AdjustedStringRemoval::default()), "asr".to_string()),
            (Arc::new(NeighbourRemoval::default()), "neighbour_removal".to_string()),
            (Arc::new(WorstJobRemoval::default()), "worst_job".to_string()),
            (
                Arc::new(ClusterRemoval::new_with_defaults(problem.clone(), environment.clone())),
                "cluster_removal".to_string(),
            ),
            (Arc::new(RandomJobRemoval::new(RuinLimits::default())), "random_job_removal_1".to_string()),
            (Arc::new(RandomRouteRemoval::default()), "random_route_removal".to_string()),
        ];
        let secondary_ruins: Vec<(Arc<dyn Ruin + Send + Sync>, String)> = vec![
            (Arc::new(CloseRouteRemoval::default()), "close_route_removal".to_string()),
            (Arc::new(RandomJobRemoval::new(RuinLimits::new(2, 8, 0.1, 2))), "random_job_removal_2".to_string()),
        ];

        // NOTE we need to wrap any of ruin methods in composite which calls restore context before recreate
        let ruins = primary_ruins
            .iter()
            .flat_map(|(outer_ruin, outer_name)| {
                secondary_ruins.iter().map(move |(inner_ruin, inner_name)| {
                    (outer_ruin.clone(), inner_ruin.clone(), format!("{}+{}", outer_name, inner_name))
                })
            })
            .map::<(Arc<dyn Ruin + Send + Sync>, String), _>(|(a, b, name)| {
                (Arc::new(CompositeRuin::new(vec![(a, 1.), (b, 1.)])), name)
            })
            .chain(primary_ruins.iter().chain(secondary_ruins.iter()).map::<(Arc<dyn Ruin + Send + Sync>, String), _>(
                |(ruin, name)| (Arc::new(CompositeRuin::new(vec![(ruin.clone(), 1.)])), name.clone()),
            ))
            .collect::<Vec<_>>();

        let static_selective = StaticSelective::create_default_mutation(problem, environment);

        let mutations: Vec<(Arc<dyn Mutation + Send + Sync>, String)> = vec![
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteBest::default()))),
                "local_exch_inter_route_best".to_string(),
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteRandom::default()))),
                "local_exch_inter_route_random".to_string(),
            ),
            (
                Arc::new(LocalSearch::new(Arc::new(ExchangeIntraRouteRandom::default()))),
                "local_exch_intra_route_random".to_string(),
            ),
            (Arc::new(LocalSearch::new(Arc::new(ExchangeSequence::default()))), "local_exch_sequence".to_string()),
            (
                Arc::new(LocalSearch::new(Arc::new(RescheduleDeparture::default()))),
                "local_reschedule_departure".to_string(),
            ),
            (Arc::new(DecomposeSearch::new(static_selective.clone(), (2, 4), 4)), "decompose_search".to_string()),
            (
                Arc::new(InfeasibleSearch::new(static_selective, 4, (0.05, 0.2), (0.05, 0.33))),
                "infeasible_search".to_string(),
            ),
        ];

        recreates
            .iter()
            .flat_map(|(recreate, recreate_name)| {
                ruins.iter().map::<(Arc<dyn Mutation + Send + Sync>, String), _>(move |(ruin, ruin_name)| {
                    (
                        Arc::new(RuinAndRecreate::new(ruin.clone(), recreate.clone())),
                        format!("{}+{}", ruin_name, recreate_name),
                    )
                })
            })
            .chain(mutations.into_iter())
            .collect::<Vec<_>>()
    }

    fn get_estimates(mutations: &[(Arc<dyn Mutation + Send + Sync>, String)]) -> ActionEstimates<SearchState> {
        let mutation_estimates = (0..mutations.len())
            .map(|idx| (SearchAction::Mutate { mutation_index: idx }, 0.))
            .collect::<HashMap<_, _>>();

        ActionEstimates::from(mutation_estimates)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchState {
    /// A state with the best known solution.
    BestKnown,
    /// A state with diverse (not the best known) solution.
    Diverse,
    /// A state with new best known solution found (major improvement).
    BestMajorImprovement,
    /// A state with new best known solution found (minor improvement).
    BestMinorImprovement,
    /// A state with improved diverse solution.
    DiverseImprovement,
    /// A state with equal or degraded solution.
    Stagnated,
}

impl State for SearchState {
    type Action = SearchAction;

    fn reward(&self) -> f64 {
        match &self {
            SearchState::BestKnown => 0.,
            SearchState::Diverse => 0.,
            SearchState::BestMajorImprovement => 1000.,
            SearchState::BestMinorImprovement => 100.,
            SearchState::DiverseImprovement => 10.,
            SearchState::Stagnated => -1.,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchAction {
    /// An action which restores solution from partially ruined, might apply an extra ruin.
    Mutate { mutation_index: usize },
}

struct SearchActionRegistry {
    pub mutations: Vec<(Arc<dyn Mutation + Send + Sync>, String)>,
}

struct SearchAgent<'a> {
    refinement_ctx: &'a RefinementContext,
    original_ctx: &'a Individual,
    registry: &'a SearchActionRegistry,
    estimates: &'a HashMap<SearchState, ActionEstimates<SearchState>>,
    state: SearchState,
    individual: Option<Individual>,
}

impl<'a> Agent<SearchState> for SearchAgent<'a> {
    fn get_state(&self) -> &SearchState {
        &self.state
    }

    fn get_actions(&self, state: &SearchState) -> ActionEstimates<SearchState> {
        self.estimates[state].clone()
    }

    fn take_action(&mut self, action: &<SearchState as State>::Action) {
        let new_individual = match action {
            SearchAction::Mutate { mutation_index } => {
                let individual = self.individual.as_ref().unwrap();
                let (mutation, _) = &self.registry.mutations[*mutation_index];

                mutation.mutate(self.refinement_ctx, individual)
            }
        };

        let compare_to_old = self.refinement_ctx.problem.objective.total_order(&new_individual, self.original_ctx);
        let compare_to_best = compare_to_best(self.refinement_ctx, &new_individual);

        self.state = match (compare_to_old, compare_to_best) {
            (_, Ordering::Less) => {
                let is_significant_change = self.refinement_ctx.population.ranked().next().map_or(
                    self.refinement_ctx.statistics.improvement_1000_ratio < 0.01,
                    |(best, _)| {
                        let objective = self.refinement_ctx.problem.objective.as_ref();
                        let distance = relative_distance(
                            objective.objectives().map(|o| o.fitness(best)),
                            objective.objectives().map(|o| o.fitness(&new_individual)),
                        );
                        distance > 0.01
                    },
                );

                if is_significant_change {
                    SearchState::BestMajorImprovement
                } else {
                    SearchState::BestMinorImprovement
                }
            }
            (Ordering::Less, _) => SearchState::DiverseImprovement,
            (_, _) => SearchState::Stagnated,
        };

        self.individual = Some(new_individual);
    }
}

impl<'a> SearchAgent<'a> {}

fn try_exchange_estimates(heuristic_simulator: &mut Simulator<SearchState>) {
    let (best_known_max, diverse_state_max) = {
        let state_estimates = heuristic_simulator.get_state_estimates();
        (
            state_estimates.get(&SearchState::BestKnown).and_then(|state| state.max_estimate()),
            state_estimates.get(&SearchState::Diverse).and_then(|state| state.max_estimate()),
        )
    };

    let is_best_known_stagnation =
        best_known_max.map_or(false, |(_, max)| compare_floats(max, 0.) != Ordering::Greater);
    let is_diverse_improvement =
        diverse_state_max.map_or(false, |(_, max)| compare_floats(max, 0.) == Ordering::Greater);

    if is_best_known_stagnation && is_diverse_improvement {
        let estimates = heuristic_simulator.get_state_estimates().get(&SearchState::Diverse).unwrap().clone();
        heuristic_simulator.set_action_estimates(SearchState::BestKnown, estimates);
    }
}

fn compare_to_best(refinement_ctx: &RefinementContext, individual: &Individual) -> Ordering {
    refinement_ctx
        .population
        .ranked()
        .next()
        .map(|(best_known, _)| refinement_ctx.problem.objective.total_order(individual, best_known))
        .unwrap_or(Ordering::Less)
}
