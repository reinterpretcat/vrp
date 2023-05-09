#[cfg(test)]
#[path = "../../tests/unit/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use super::*;
use crate::algorithms::math::{relative_distance, RemedianUsize};
use crate::algorithms::mdp::*;
use crate::utils::compare_floats;
use crate::Timer;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

/// A collection of heuristic search operators with their name and initial weight.
pub type HeuristicSearchOperators<C, O, S> =
    Vec<(Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>, String, f64)>;

/// A collection of heuristic diversify operators.
pub type HeuristicDiversifyOperators<C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// An experimental dynamic selective hyper heuristic which selects inner heuristics
/// based on how they work during the search. The selection process is modeled by
/// Markov Decision Process.
pub struct DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_simulator: Simulator<SearchState>,
    initial_estimates: StateEstimates<SearchState>,
    action_registry: SearchActionRegistry<C, O, S>,
    diversify_operators: HeuristicDiversifyOperators<C, O, S>,
    tracker: HeuristicTracker,
}

impl<C, O, S> HyperHeuristic for DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let agent = self.heuristic_simulator.run_episode(
            SearchAgent {
                heuristic_ctx,
                original: solution,
                registry: &self.action_registry,
                estimates: &self.initial_estimates,
                tracker: &self.tracker,
                state: Self::compare_solution(heuristic_ctx, solution),
                solution: Some(solution.deep_copy()),
                samples: Vec::default(),
            },
            get_state_reducer(),
        );

        let (individuals, runtimes) =
            agent.solution.map(|solution| (vec![solution], agent.samples)).unwrap_or_default();

        self.update_simulator(heuristic_ctx, runtimes);

        individuals
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        let agents = solutions
            .into_iter()
            .map(|solution| SearchAgent {
                heuristic_ctx,
                original: solution,
                registry: &self.action_registry,
                estimates: &self.initial_estimates,
                tracker: &self.tracker,
                state: Self::compare_solution(heuristic_ctx, solution),
                solution: Some(solution.deep_copy()),
                samples: Vec::default(),
            })
            .collect();

        let (individuals, runtimes) = self
            .heuristic_simulator
            // NOTE use parallelism setting
            .run_episodes(agents, heuristic_ctx.environment().parallelism.clone(), get_state_reducer())
            .into_iter()
            .filter_map(|agent| agent.solution.map(|solution| (solution, agent.samples)))
            .fold((Vec::new(), Vec::new()), |mut acc, (solution, runtime)| {
                acc.0.push(solution);
                acc.1.extend(runtime.into_iter());
                acc
            });

        self.update_simulator(heuristic_ctx, runtimes);

        individuals
    }

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let probability = get_diversify_probability(heuristic_ctx);
        if heuristic_ctx.environment().random.is_hit(probability) {
            diversify_solution(heuristic_ctx, solution, self.diversify_operators.as_slice())
        } else {
            Vec::default()
        }
    }

    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        diversify_solutions(heuristic_ctx, solutions, self.diversify_operators.as_slice())
    }
}

impl<C, O, S> DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `DynamicSelective` heuristic.
    pub fn new(
        search_operators: HeuristicSearchOperators<C, O, S>,
        diversify_operators: HeuristicDiversifyOperators<C, O, S>,
        environment: &Environment,
    ) -> Self {
        let operator_estimates = search_operators
            .iter()
            .enumerate()
            .map(|(heuristic_idx, (_, _, weight))| (SearchAction::Search { heuristic_idx }, *weight))
            .collect::<HashMap<_, _>>();
        let operator_estimates = ActionEstimates::from(operator_estimates);

        Self {
            heuristic_simulator: Simulator::new(
                create_learning_strategy(0.),
                create_policy_strategy(0., environment.random.clone()),
            ),
            initial_estimates: vec![
                (SearchState::BestKnown(Default::default()), operator_estimates.clone()),
                (SearchState::Diverse(Default::default()), operator_estimates),
                (SearchState::BestMajorImprovement(Default::default()), Default::default()),
                (SearchState::BestMinorImprovement(Default::default()), Default::default()),
                (SearchState::DiverseImprovement(Default::default()), Default::default()),
                (SearchState::Stagnated(Default::default()), Default::default()),
            ]
            .into_iter()
            .collect(),
            action_registry: SearchActionRegistry { heuristics: search_operators },
            diversify_operators,
            tracker: HeuristicTracker {
                total_median: RemedianUsize::new(11, |a, b| a.cmp(b)),
                telemetry: Default::default(),
                is_experimental: environment.is_experimental,
            },
        }
    }

    fn compare_solution(heuristic_ctx: &C, solution: &S) -> SearchState {
        match compare_to_best(heuristic_ctx, solution) {
            Ordering::Greater => SearchState::Diverse(Default::default()),
            _ => SearchState::BestKnown(Default::default()),
        }
    }

    fn update_simulator(&mut self, heuristic_ctx: &C, samples: Vec<SearchSample>) {
        try_exchange_estimates(&mut self.heuristic_simulator);

        // update learning and policy strategies to move from more exploration at the beginning to exploitation in the end
        let termination_estimate = heuristic_ctx.statistics().termination_estimate;
        let random = heuristic_ctx.environment().random.clone();
        self.heuristic_simulator.set_learning_strategy(create_learning_strategy(termination_estimate));
        self.heuristic_simulator.set_policy_strategy(create_policy_strategy(termination_estimate, random));

        samples.into_iter().for_each(|sample| {
            let estimate = self
                .heuristic_simulator
                .get_state_estimates()
                .get(&sample.old_state)
                .and_then(|action_estimates| action_estimates.data().get(&sample.action))
                .cloned()
                .unwrap_or_default();
            self.tracker.observe(heuristic_ctx.statistics().generation, estimate, sample);
        });
    }
}

#[derive(Default, Clone)]
struct MedianRatio {
    pub ratio: f64,
}

impl Hash for MedianRatio {
    fn hash<H: Hasher>(&self, state: &mut H) {
        0.hash(state)
    }
}

impl PartialEq for MedianRatio {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for MedianRatio {}

impl MedianRatio {
    pub fn eval(&self, value: f64) -> f64 {
        let ratio = self.ratio.clamp(0.5, 2.);

        match (ratio, compare_floats(value, 0.)) {
            (ratio, _) if ratio < 1.001 => value,
            (ratio, Ordering::Equal) => -2. * ratio,
            (ratio, Ordering::Less) => value * ratio,
            (ratio, Ordering::Greater) => value / ratio,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchState {
    /// A state with the best known solution.
    BestKnown(MedianRatio),
    /// A state with diverse (not the best known) solution.
    Diverse(MedianRatio),
    /// A state with new best known solution found (major improvement).
    BestMajorImprovement(MedianRatio),
    /// A state with new best known solution found (minor improvement).
    BestMinorImprovement(MedianRatio),
    /// A state with improved diverse solution.
    DiverseImprovement(MedianRatio),
    /// A state with equal or degraded solution.
    Stagnated(MedianRatio),
}

impl State for SearchState {
    type Action = SearchAction;

    fn reward(&self) -> f64 {
        match &self {
            SearchState::BestKnown(median_ratio) => median_ratio.eval(0.),
            SearchState::Diverse(median_ratio) => median_ratio.eval(0.),
            SearchState::BestMajorImprovement(median_ratio) => median_ratio.eval(1000.),
            SearchState::BestMinorImprovement(median_ratio) => median_ratio.eval(100.),
            SearchState::DiverseImprovement(median_ratio) => median_ratio.eval(10.),
            SearchState::Stagnated(median_ratio) => median_ratio.eval(-1.),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchAction {
    /// An action which performs the search.
    Search { heuristic_idx: usize },
}

struct SearchActionRegistry<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    pub heuristics: HeuristicSearchOperators<C, O, S>,
}

struct SearchSample {
    /// Name of heuristic used.
    pub name: String,
    /// Duration of the search.
    pub duration: Duration,
    /// Old state of the search.
    pub old_state: SearchState,
    /// Final state of the search.
    pub new_state: SearchState,
    /// Action taken.
    pub action: SearchAction,
}

struct SearchAgent<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx: &'a C,
    registry: &'a SearchActionRegistry<C, O, S>,
    estimates: &'a StateEstimates<SearchState>,
    tracker: &'a HeuristicTracker,
    state: SearchState,
    original: &'a S,
    solution: Option<S>,
    samples: Vec<SearchSample>,
}

impl<'a, C, O, S> Agent<SearchState> for SearchAgent<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn get_state(&self) -> &SearchState {
        &self.state
    }

    fn get_actions(&self, state: &SearchState) -> ActionEstimates<SearchState> {
        self.estimates[state].clone()
    }

    fn take_action(&mut self, action: &<SearchState as State>::Action) {
        let (new_solution, duration, name) = match action {
            SearchAction::Search { heuristic_idx } => {
                let solution = self.solution.as_ref().unwrap();
                let (heuristic, name, _) = &self.registry.heuristics[*heuristic_idx];

                let (new_solution, duration) =
                    Timer::measure_duration(|| heuristic.search(self.heuristic_ctx, solution));

                (new_solution, duration, name.to_string())
            }
        };

        let objective = self.heuristic_ctx.objective();

        let compare_to_old = objective.total_order(&new_solution, self.original);
        let compare_to_best = compare_to_best(self.heuristic_ctx, &new_solution);

        let ratio = MedianRatio {
            ratio: self.tracker.approx_median().map_or(1., |median| {
                if median == 0 {
                    1.
                } else {
                    duration.as_millis() as f64 / median as f64
                }
            }),
        };

        let old_state = self.state.clone();
        self.state = match (compare_to_old, compare_to_best) {
            (_, Ordering::Less) => {
                let is_significant_change = self.heuristic_ctx.population().ranked().next().map_or(
                    self.heuristic_ctx.statistics().improvement_1000_ratio < 0.01,
                    |(best, _)| {
                        let distance = relative_distance(objective.fitness(best), objective.fitness(&new_solution));
                        distance > 0.01
                    },
                );

                if is_significant_change {
                    SearchState::BestMajorImprovement(ratio)
                } else {
                    SearchState::BestMinorImprovement(ratio)
                }
            }
            (Ordering::Less, _) => SearchState::DiverseImprovement(ratio),
            (_, _) => SearchState::Stagnated(ratio),
        };

        self.solution = Some(new_solution);
        self.samples.push(SearchSample {
            name,
            duration,
            old_state,
            new_state: self.state.clone(),
            action: action.clone(),
        })
    }
}

impl<C, O, S> Display for DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.tracker.telemetry.is_empty() {
            return Ok(());
        }

        f.write_fmt(format_args!("name,generation,duration,estimation,state\n"))?;
        self.tracker.telemetry.iter().try_for_each(|(name, entries)| {
            entries.iter().try_for_each(|(generation, duration, estimate, state)| {
                let state = match state {
                    SearchState::BestKnown(_) => unreachable!(),
                    SearchState::Diverse(_) => unreachable!(),
                    SearchState::BestMajorImprovement(_) => "best_major",
                    SearchState::BestMinorImprovement(_) => "best_minor",
                    SearchState::DiverseImprovement(_) => "diverse",
                    SearchState::Stagnated(_) => "stagnated",
                };
                f.write_fmt(format_args!("{},{},{},{},{}\n", name, generation, duration.as_millis(), estimate, state))
            })
        })
    }
}

fn try_exchange_estimates(heuristic_simulator: &mut Simulator<SearchState>) {
    let (best_known_max, diverse_state_max) = {
        let state_estimates = heuristic_simulator.get_state_estimates();
        (
            state_estimates.get(&SearchState::BestKnown(Default::default())).and_then(|state| state.max_estimate()),
            state_estimates.get(&SearchState::Diverse(Default::default())).and_then(|state| state.max_estimate()),
        )
    };

    let is_best_known_stagnation =
        best_known_max.map_or(false, |(_, max)| compare_floats(max, 0.) != Ordering::Greater);
    let is_diverse_improvement =
        diverse_state_max.map_or(false, |(_, max)| compare_floats(max, 0.) == Ordering::Greater);

    if is_best_known_stagnation && is_diverse_improvement {
        let estimates =
            heuristic_simulator.get_state_estimates().get(&SearchState::Diverse(Default::default())).unwrap().clone();
        heuristic_simulator.set_action_estimates(SearchState::BestKnown(Default::default()), estimates);
    }
}

fn compare_to_best<C, O, S>(heuristic_ctx: &C, solution: &S) -> Ordering
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx
        .population()
        .ranked()
        .next()
        .map(|(best_known, _)| heuristic_ctx.objective().total_order(solution, best_known))
        .unwrap_or(Ordering::Less)
}

struct HeuristicTracker {
    total_median: RemedianUsize,
    telemetry: HashMap<String, Vec<(usize, Duration, f64, SearchState)>>,
    is_experimental: bool,
}

impl HeuristicTracker {
    pub fn observe(&mut self, generation: usize, estimate: f64, sample: SearchSample) {
        self.total_median.add_observation(sample.duration.as_millis() as usize);
        // NOTE track heuristic telemetry only for experimental mode (performance)
        if self.is_experimental {
            self.telemetry.entry(sample.name).or_default().push((
                generation,
                sample.duration,
                estimate,
                sample.new_state,
            ));
        }
    }

    pub fn approx_median(&self) -> Option<usize> {
        self.total_median.approx_median()
    }
}

fn create_learning_strategy(termination_estimate: f64) -> Box<dyn LearningStrategy<SearchState> + Send + Sync> {
    // https://www.wolframalpha.com/input?i=plot++%281%2F%284%2Be%5E%28-4*%28x+-+0.25%29%29%29%29%2C+x%3D0+to+1
    let x = termination_estimate.clamp(0., 1.);
    let alpha = 1. / (4. + std::f64::consts::E.powf(-4. * (x - 0.25)));

    Box::new(MonteCarlo::new(alpha))
}

fn create_policy_strategy(
    termination_estimate: f64,
    random: Arc<dyn Random + Send + Sync>,
) -> Box<dyn PolicyStrategy<SearchState> + Send + Sync> {
    // https://www.wolframalpha.com/input?i=plot+0.2*+%281+-+1%2F%281%2Be%5E%28-4+*%28x+-+0.25%29%29%29%29%2C+x%3D0+to+1
    let x = termination_estimate.clamp(0., 1.);
    let epsilon = 0.2 * (1. - 1. / (1. + std::f64::consts::E.powf(-4. * (x - 0.25))));

    Box::new(EpsilonWeighted::new(epsilon, random))
}

fn get_state_reducer() -> impl Fn(&SearchState, &[f64]) -> f64 {
    |state, values| match state {
        SearchState::BestKnown { .. } => values.iter().max_by(|a, b| compare_floats(**a, **b)).cloned().unwrap_or(0.),
        _ => values.iter().sum::<f64>() / values.len() as f64,
    }
}
