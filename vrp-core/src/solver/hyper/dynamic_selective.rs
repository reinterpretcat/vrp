use crate::algorithms::mdp::{ActionsEstimate, Agent, Simulator, State};
use crate::algorithms::nsga2::Objective;
use crate::solver::hyper::HyperHeuristic;
use crate::solver::mutation::{Mutation, Ruin};
use crate::solver::population::Individual;
use crate::solver::RefinementContext;
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

pub struct DynamicSelective {
    heuristic_simulator: Simulator<SearchState>,
    action_registry: SearchActionRegistry,
    initial_estimates: HashMap<SearchState, ActionsEstimate<SearchState>>,
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
