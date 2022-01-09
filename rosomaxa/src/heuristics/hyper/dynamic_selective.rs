use super::*;
use crate::algorithms::math::relative_distance;
use crate::algorithms::mdp::*;
use crate::algorithms::nsga2::Objective;
use crate::utils::{compare_floats, Random};
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

/// An experimental dynamic selective hyper heuristic which selects inner heuristics
/// based on how they work during the search. The selection process is modeled by
/// Markov Decision Process.
pub struct DynamicSelective<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    heuristic_simulator: Simulator<SearchState>,
    initial_estimates: HashMap<SearchState, ActionEstimates<SearchState>>,
    action_registry: SearchActionRegistry<C, O, P, S>,
}

impl<C, O, P, S> HyperHeuristic for DynamicSelective<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        let registry = &self.action_registry;
        let estimates = &self.initial_estimates;

        let agents = solutions
            .into_iter()
            .map(|solution| {
                Box::new(SearchAgent {
                    heuristic_ctx,
                    original: solution,
                    registry,
                    estimates,
                    state: match compare_to_best(heuristic_ctx, solution) {
                        Ordering::Greater => SearchState::Diverse,
                        _ => SearchState::BestKnown,
                    },
                    solution: Some(solution.deep_copy()),
                })
            })
            .collect();

        let individuals = self
            .heuristic_simulator
            .run_episodes(agents, heuristic_ctx.environment().parallelism.clone(), |state, values| match state {
                SearchState::BestKnown => values.iter().max_by(|a, b| compare_floats(**a, **b)).cloned().unwrap_or(0.),
                _ => values.iter().sum::<f64>() / values.len() as f64,
            })
            .into_iter()
            .filter_map(|agent| agent.solution)
            .collect();

        try_exchange_estimates(&mut self.heuristic_simulator);

        individuals
    }
}

impl<C, O, P, S> DynamicSelective<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `DynamicSelective`.
    pub fn new_with_defaults(
        operators: Vec<(Arc<dyn HeuristicOperator<Context = C, Solution = S> + Send + Sync>, String)>,
        random: Arc<dyn Random + Send + Sync>,
    ) -> Self {
        let operator_estimates = (0..operators.len())
            .map(|heuristic_idx| (SearchAction::Search { heuristic_idx }, 0.))
            .collect::<HashMap<_, _>>();

        let operator_estimates = ActionEstimates::from(operator_estimates);

        Self {
            heuristic_simulator: Simulator::new(
                Box::new(MonteCarlo::new(0.1)),
                Box::new(EpsilonWeighted::new(0.1, random)),
            ),
            initial_estimates: vec![
                (SearchState::BestKnown, operator_estimates.clone()),
                (SearchState::Diverse, operator_estimates),
                (SearchState::BestMajorImprovement, Default::default()),
                (SearchState::BestMinorImprovement, Default::default()),
                (SearchState::DiverseImprovement, Default::default()),
                (SearchState::Stagnated, Default::default()),
            ]
            .into_iter()
            .collect(),
            action_registry: SearchActionRegistry { heuristics: operators },
        }
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
    /// An action which performs the search.
    Search { heuristic_idx: usize },
}

struct SearchActionRegistry<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    pub heuristics: Vec<(Arc<dyn HeuristicOperator<Context = C, Solution = S> + Send + Sync>, String)>,
}

struct SearchAgent<'a, C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    heuristic_ctx: &'a C,
    registry: &'a SearchActionRegistry<C, O, P, S>,
    estimates: &'a HashMap<SearchState, ActionEstimates<SearchState>>,
    state: SearchState,
    original: &'a S,
    solution: Option<S>,
}

impl<'a, C, O, P, S> Agent<SearchState> for SearchAgent<'a, C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    fn get_state(&self) -> &SearchState {
        &self.state
    }

    fn get_actions(&self, state: &SearchState) -> ActionEstimates<SearchState> {
        self.estimates[state].clone()
    }

    fn take_action(&mut self, action: &<SearchState as State>::Action) {
        let new_solution = match action {
            SearchAction::Search { heuristic_idx } => {
                let solution = self.solution.as_ref().unwrap();
                let (heuristic, _) = &self.registry.heuristics[*heuristic_idx];

                heuristic.search(self.heuristic_ctx, solution)
            }
        };

        let objective = self.heuristic_ctx.objective();

        let compare_to_old = objective.total_order(&new_solution, self.original);
        let compare_to_best = compare_to_best(self.heuristic_ctx, &new_solution);

        self.state = match (compare_to_old, compare_to_best) {
            (_, Ordering::Less) => {
                let is_significant_change = self.heuristic_ctx.population().ranked().next().map_or(
                    self.heuristic_ctx.statistics().improvement_1000_ratio < 0.01,
                    |(best, _)| {
                        let distance = relative_distance(
                            objective.objectives().map(|o| o.fitness(best)),
                            objective.objectives().map(|o| o.fitness(&new_solution)),
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

        self.solution = Some(new_solution);
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

fn compare_to_best<C, O, P, S>(heuristic_ctx: &C, solution: &S) -> Ordering
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    heuristic_ctx
        .population()
        .ranked()
        .next()
        .map(|(best_known, _)| heuristic_ctx.objective().total_order(solution, best_known))
        .unwrap_or(Ordering::Less)
}
