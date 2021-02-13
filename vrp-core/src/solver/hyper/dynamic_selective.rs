#[cfg(test)]
#[path = "../../../tests/unit/solver/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use crate::algorithms::mdp::*;
use crate::algorithms::nsga2::Objective;
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
        let mutations = Self::get_mutations(problem);
        let mutation_estimates = Self::get_estimates(mutations.clone());

        Self {
            heuristic_simulator: Simulator::new(
                Box::new(MonteCarlo::new(0.01)),
                Box::new(EpsilonWeighted::new(0.1, environment.random.clone())),
            ),
            initial_estimates: vec![
                (SearchState::BestKnown, mutation_estimates.clone()),
                (SearchState::Diverse, mutation_estimates.clone()),
                (SearchState::NewBest, Default::default()),
                (SearchState::Improved, Default::default()),
                (SearchState::Stagnated, Default::default()),
            ]
            .into_iter()
            .collect(),
            action_registry: SearchActionRegistry { mutations },
        }
    }

    fn get_mutations(problem: Arc<Problem>) -> Vec<Arc<dyn Mutation + Send + Sync>> {
        let recreates: Vec<Arc<dyn Recreate + Send + Sync>> = vec![
            Arc::new(RecreateWithSkipBest::new(1, 2)),
            Arc::new(RecreateWithSkipBest::new(3, 4)),
            Arc::new(RecreateWithSkipBest::new(4, 8)),
            Arc::new(RecreateWithRegret::new(2, 3)),
            Arc::new(RecreateWithCheapest::default()),
            Arc::new(RecreateWithPerturbation::default()),
            Arc::new(RecreateWithPerturbation::new(0.1, 0.9, 1.1)),
            Arc::new(RecreateWithGaps::default()),
            Arc::new(RecreateWithBlinks::<SingleDimLoad>::default()),
            Arc::new(RecreateWithBlinks::<MultiDimLoad>::default()),
            Arc::new(RecreateWithFarthest::default()),
            Arc::new(RecreateWithNearestNeighbor::default()),
        ];

        let simple_ruins: Vec<Arc<dyn Ruin + Send + Sync>> = vec![
            Arc::new(AdjustedStringRemoval::default()),
            Arc::new(NeighbourRemoval::default()),
            Arc::new(WorstJobRemoval::default()),
            Arc::new(ClusterRemoval::new_with_defaults(problem.clone())),
        ];
        let extra_ruins: Vec<Arc<dyn Ruin + Send + Sync>> = vec![
            Arc::new(CloseRouteRemoval::default()),
            Arc::new(RandomRouteRemoval::default()),
            Arc::new(RandomJobRemoval::new(JobRemovalLimit::new(1, 2, 0.1))),
            Arc::new(RandomJobRemoval::new(JobRemovalLimit::new(2, 8, 0.1))),
            Arc::new(RandomJobRemoval::new(JobRemovalLimit::default())),
            Arc::new(RandomJobRemoval::new(JobRemovalLimit::new(16, 32, 0.2))),
        ];
        let combine_ruins: Vec<Arc<dyn Ruin + Send + Sync>> = vec![Arc::new(CloseRouteRemoval::default())];

        // NOTE we need to wrap any of ruin methods in composite which calls restore context before recreate
        let ruins = simple_ruins
            .iter()
            .flat_map(|outer_ruin| combine_ruins.iter().map(move |inner_ruin| (outer_ruin.clone(), inner_ruin.clone())))
            .map::<Arc<dyn Ruin + Send + Sync>, _>(|(a, b)| Arc::new(CompositeRuin::new(vec![(a, 1.), (b, 1.)])))
            .chain(
                simple_ruins.iter().chain(extra_ruins.iter()).map::<Arc<dyn Ruin + Send + Sync>, _>(|ruin| {
                    Arc::new(CompositeRuin::new(vec![(ruin.clone(), 1.)]))
                }),
            )
            .collect::<Vec<_>>();

        let mutations: Vec<Arc<dyn Mutation + Send + Sync>> = vec![
            Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteBest::default()))),
            Arc::new(LocalSearch::new(Arc::new(ExchangeInterRouteRandom::default()))),
            Arc::new(LocalSearch::new(Arc::new(ExchangeIntraRouteRandom::default()))),
            Arc::new(DecomposeSearch::new(StaticSelective::create_default_mutation(problem), (2, 4), 4)),
        ];

        let mutations = recreates
            .iter()
            .flat_map(|recreate| {
                ruins.iter().map::<Arc<dyn Mutation + Send + Sync>, _>(move |ruin| {
                    Arc::new(RuinAndRecreate::new(ruin.clone(), recreate.clone()))
                })
            })
            .chain(mutations.into_iter())
            .collect::<Vec<_>>();

        mutations
    }

    fn get_estimates(mutations: Vec<Arc<dyn Mutation + Send + Sync>>) -> ActionEstimates<SearchState> {
        let mutation_estimates = (0..mutations.len())
            // NOTE default value should be less than reward for Stagnated state
            .map(|idx| (SearchAction::Mutate { mutation_index: idx }, -2.))
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
    /// A state with new best known solution found.
    NewBest,
    /// A state with the same equal solution.
    Improved,
    /// A state with equal or degraded solution.
    Stagnated,
}

impl State for SearchState {
    type Action = SearchAction;

    fn reward(&self) -> f64 {
        match &self {
            SearchState::BestKnown => 0.,
            SearchState::Diverse => 0.,
            SearchState::NewBest => 100.,
            SearchState::Improved => 10.,
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
    pub mutations: Vec<Arc<dyn Mutation + Send + Sync>>,
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
        let mut new_individual = match action {
            SearchAction::Mutate { mutation_index } => {
                let individual = self.individual.as_ref().unwrap();
                let mutation = &self.registry.mutations[*mutation_index];

                mutation.mutate(self.refinement_ctx, individual)
            }
        };

        new_individual.restore();

        let compare_to_old = self.refinement_ctx.problem.objective.total_order(&new_individual, self.original_ctx);
        let compare_to_best = compare_to_best(self.refinement_ctx, &new_individual);

        self.state = match (compare_to_old, compare_to_best) {
            (_, Ordering::Less) => SearchState::NewBest,
            (Ordering::Less, _) => SearchState::Improved,
            (_, _) => SearchState::Stagnated,
        };

        self.individual = Some(new_individual);
    }
}

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
        .map(|(best_known, _)| refinement_ctx.problem.objective.total_order(&individual, best_known))
        .unwrap_or(Ordering::Less)
}
