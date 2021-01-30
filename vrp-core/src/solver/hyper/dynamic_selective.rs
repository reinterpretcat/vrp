use crate::algorithms::mdp::{ActionsEstimate, Agent, EpsilonGreedy, PolicyStrategy, QLearning, Simulator, State};
use crate::algorithms::nsga2::Objective;
use crate::models::common::SingleDimLoad;
use crate::models::Problem;
use crate::solver::hyper::{HyperHeuristic, StaticSelective};
use crate::solver::mutation::*;
use crate::solver::population::Individual;
use crate::solver::RefinementContext;
use crate::utils::Environment;
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::iter::once;
use std::sync::Arc;

// TODO limit ruin by max unassigned/required jobs

pub struct DynamicSelective {
    heuristic_simulator: Simulator<SearchState>,
    initial_estimates: HashMap<SearchState, ActionsEstimate<SearchState>>,
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

        self.heuristic_simulator
            .run_episodes(agents, refinement_ctx.environment.parallelism.clone(), |values| {
                values.iter().sum::<f64>() / values.len() as f64
            })
            .into_iter()
            .filter_map(|agent| agent.individual)
            .collect()
    }
}

impl DynamicSelective {
    /// Creates a new instance of `DynamicSelective`.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        let recreates: Vec<Arc<dyn Recreate + Send + Sync>> = vec![
            Arc::new(RecreateWithSkipBest::new(1, 2)),
            Arc::new(RecreateWithRegret::new(2, 3)),
            Arc::new(RecreateWithCheapest::default()),
            Arc::new(RecreateWithPerturbation::default()),
            Arc::new(RecreateWithSkipBest::new(3, 4)),
            Arc::new(RecreateWithGaps::default()),
            Arc::new(RecreateWithBlinks::<SingleDimLoad>::default()),
            Arc::new(RecreateWithFarthest::default()),
            Arc::new(RecreateWithSkipBest::new(4, 8)),
            Arc::new(RecreateWithNearestNeighbor::default()),
        ];

        let ruins: Vec<Arc<dyn Ruin + Send + Sync>> = vec![
            Arc::new(AdjustedStringRemoval::default()),
            Arc::new(NeighbourRemoval::new(JobRemovalLimit::new(2, 8, 0.1))),
            Arc::new(WorstJobRemoval::default()),
            Arc::new(NeighbourRemoval::default()),
            Arc::new(ClusterRemoval::new_with_defaults(problem.clone())),
            Arc::new(RandomRouteRemoval::default()),
            Arc::new(RandomJobRemoval::new(JobRemovalLimit::default())),
        ];

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

        Self {
            heuristic_simulator: Simulator::new(
                Box::new(QLearning::new(0.1, 0.02)),
                Box::new(EpsilonGreedy::new(0.2, environment.random.clone())),
            ),
            initial_estimates: vec![].into_iter().collect(),
            action_registry: SearchActionRegistry { ruins, mutations },
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchState {
    /// A state with the best known solution.
    BestKnown,
    /// A state with diverse (not the best known) solution.
    Diverse,
    /// A state with partially ruined solution.
    Ruined,
    /// A state with new best known solution found.
    NewBest,
    /// A state with improved from diverse solution.
    Improved,
    /// A state with degraded solution.
    Degraded,
}

impl State for SearchState {
    type Action = SearchAction;

    fn reward(&self) -> f64 {
        match &self {
            SearchState::BestKnown => 0.,
            SearchState::Diverse => 0.,
            SearchState::Ruined => 0.,
            SearchState::NewBest => 100.,
            SearchState::Improved => 1.,
            SearchState::Degraded => -10.,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchAction {
    /// An action which only ruins solution.
    Ruin { ruin_index: usize },
    /// An action which restores solution from partially ruined, might apply an extra ruin.
    Mutate { mutation_index: usize },
}

struct SearchActionRegistry {
    pub ruins: Vec<Arc<dyn Ruin + Send + Sync>>,
    pub mutations: Vec<Arc<dyn Mutation + Send + Sync>>,
}

struct SearchAgent<'a> {
    refinement_ctx: &'a RefinementContext,
    registry: &'a SearchActionRegistry,
    estimates: &'a HashMap<SearchState, ActionsEstimate<SearchState>>,
    state: SearchState,
    individual: Option<Individual>,
}

impl<'a> Agent<SearchState> for SearchAgent<'a> {
    fn get_state(&self) -> &SearchState {
        &self.state
    }

    fn get_actions(&self, state: &SearchState) -> ActionsEstimate<SearchState> {
        self.estimates[state].clone()
    }

    fn take_action(&mut self, action: &<SearchState as State>::Action) {
        let new_individual = match action {
            SearchAction::Ruin { ruin_index } => {
                let individual = std::mem::replace(&mut self.individual, None).expect("no insertion ctx");
                let ruin = &self.registry.ruins[*ruin_index];

                // TODO do we need to call accept solution in the end?
                ruin.run(self.refinement_ctx, individual)
            }
            SearchAction::Mutate { mutation_index } => {
                let indvidual = self.individual.as_ref().unwrap();
                let mutation = &self.registry.mutations[*mutation_index];

                mutation.mutate(self.refinement_ctx, indvidual)
            }
        };

        self.state = if let Some(old_individual) = self.individual.as_ref() {
            let compare_to_old = self.refinement_ctx.problem.objective.total_order(&new_individual, old_individual);
            let compare_to_best = compare_to_best(self.refinement_ctx, &new_individual);

            match (compare_to_old, compare_to_best) {
                (_, Ordering::Less) => SearchState::NewBest,
                (_, Ordering::Equal) => SearchState::Improved,
                (Ordering::Less, _) => SearchState::Improved,
                (_, _) => SearchState::Degraded,
            }
        } else {
            SearchState::Ruined
        };

        self.individual = Some(new_individual);
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
