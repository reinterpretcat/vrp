#[cfg(test)]
#[path = "../../tests/unit/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use super::*;
use crate::algorithms::math::RemedianUsize;
use crate::algorithms::rl::{SlotAction, SlotFeedback, SlotMachine};
use crate::utils::{compare_floats, random_argmax, DefaultDistributionSampler};
use crate::Timer;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::Hash;
use std::iter::once;
use std::sync::Arc;

/// A collection of heuristic search operators with their name and initial weight.
pub type HeuristicSearchOperators<F, C, O, S> = Vec<(
    Arc<dyn HeuristicSearchOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync>,
    String,
    f64,
)>;

/// A collection of heuristic diversify operators.
pub type HeuristicDiversifyOperators<F, C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// An experimental dynamic selective hyper heuristic which selects inner heuristics
/// based on how they work during the search. The selection process is modeled using reinforcement
/// learning technics.
pub struct DynamicSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    agent: SearchAgent<'static, F, C, O, S>,
    diversify_operators: HeuristicDiversifyOperators<F, C, O, S>,
}

impl<F, C, O, S> HyperHeuristic for DynamicSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F> + 'static,
{
    type Fitness = F;
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let generation = heuristic_ctx.statistics().generation;
        let feedback = self.agent.search(heuristic_ctx, solution);

        self.agent.update(generation, &feedback);

        vec![feedback.solution]
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        let feedbacks = parallel_into_collect(solutions.iter().enumerate().collect(), |(idx, solution)| {
            heuristic_ctx
                .environment()
                .parallelism
                .thread_pool_execute(idx, || self.agent.search(heuristic_ctx, solution))
        });

        let generation = heuristic_ctx.statistics().generation;
        feedbacks.iter().for_each(|feedback| {
            self.agent.update(generation, feedback);
        });

        self.agent.save_params(generation);

        feedbacks.into_iter().map(|feedback| feedback.solution).collect()
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

impl<F, C, O, S> DynamicSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F> + 'static,
{
    /// Creates a new instance of `DynamicSelective` heuristic.
    pub fn new(
        search_operators: HeuristicSearchOperators<F, C, O, S>,
        diversify_operators: HeuristicDiversifyOperators<F, C, O, S>,
        environment: &Environment,
    ) -> Self {
        Self { agent: SearchAgent::new(search_operators, environment), diversify_operators }
    }
}

type SlotMachines<'a, F, C, O, S> =
    Vec<(SlotMachine<SearchAction<'a, F, C, O, S>, DefaultDistributionSampler>, String)>;

#[derive(PartialEq, Eq, Hash, Clone)]
enum SearchState {
    BestKnown,
    Diverse,
}

impl Display for SearchState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchState::BestKnown => f.write_str("best"),
            SearchState::Diverse => f.write_str("diverse"),
        }
    }
}

struct SearchFeedback<S> {
    sample: SearchSample,
    slot_idx: usize,
    solution: S,
}

impl<S> SlotFeedback for SearchFeedback<S> {
    fn reward(&self) -> f64 {
        self.sample.reward
    }
}

struct SearchAction<'a, F, C, O, S> {
    operator:
        Arc<dyn HeuristicSearchOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync + 'a>,
    operator_name: String,
}

impl<'a, F, C, O, S> Clone for SearchAction<'a, F, C, O, S> {
    fn clone(&self) -> Self {
        Self { operator: self.operator.clone(), operator_name: self.operator_name.clone() }
    }
}

impl<'a, F, C, O, S> SlotAction for SearchAction<'a, F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F> + 'a,
{
    type Context = SearchContext<'a, F, C, O, S>;
    type Feedback = SearchFeedback<S>;

    fn take(&self, context: Self::Context) -> Self::Feedback {
        let (new_solution, duration) =
            Timer::measure_duration(|| self.operator.search(context.heuristic_ctx, context.solution));

        let is_new_best = compare_to_best(context.heuristic_ctx, &new_solution) == Ordering::Less;
        let duration = duration.as_millis() as usize;

        let base_reward = estimate_distance_reward(context.heuristic_ctx, context.solution, &new_solution);
        let reward_multiplier = estimate_reward_perf_multiplier(&context, duration, is_new_best);
        let reward = base_reward * reward_multiplier;

        let to = if is_new_best { SearchState::BestKnown } else { SearchState::Diverse };
        let transition = (context.from, to);

        let sample = SearchSample { name: self.operator_name.clone(), duration, reward, transition };

        SearchFeedback { sample, slot_idx: context.slot_idx, solution: new_solution }
    }
}

struct SearchContext<'a, F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    heuristic_ctx: &'a C,
    from: SearchState,
    slot_idx: usize,
    solution: &'a S,
    approx_median: Option<usize>,
}

struct SearchAgent<'a, F, C, O, S> {
    slot_machines: HashMap<SearchState, SlotMachines<'a, F, C, O, S>>,
    tracker: HeuristicTracker,
    random: Arc<dyn Random + Send + Sync>,
}

impl<'a, F, C, O, S> SearchAgent<'a, F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F> + 'a,
{
    pub fn new(search_operators: HeuristicSearchOperators<F, C, O, S>, environment: &Environment) -> Self {
        let slot_machines = search_operators
            .into_iter()
            .map(|(operator, name, _)| {
                // TODO use initial weight as prior mean estimation?
                (
                    SlotMachine::new(
                        1.,
                        SearchAction { operator, operator_name: name.to_string() },
                        DefaultDistributionSampler::new(environment.random.clone()),
                    ),
                    name,
                )
            })
            .collect::<Vec<_>>();

        let slot_machines = once((SearchState::BestKnown, slot_machines.clone()))
            .chain(once((SearchState::Diverse, slot_machines)))
            .collect();

        Self {
            slot_machines,
            tracker: HeuristicTracker {
                total_median: RemedianUsize::new(11, |a, b| a.cmp(b)),
                search_telemetry: Default::default(),
                heuristic_telemetry: Default::default(),
                is_experimental: environment.is_experimental,
            },
            random: environment.random.clone(),
        }
    }

    /// Picks relevant search operator based on learnings and runs the search.
    pub fn search(&self, heuristic_ctx: &C, solution: &S) -> SearchFeedback<S> {
        let from = if matches!(compare_to_best(heuristic_ctx, solution), Ordering::Equal) {
            SearchState::BestKnown
        } else {
            SearchState::Diverse
        };

        let (slot_idx, slot_machine) = self
            .slot_machines
            .get(&from)
            .and_then(|slots| {
                random_argmax(slots.iter().map(|(slot, _)| slot.sample()), self.random.as_ref())
                    .and_then(|slot_idx| slots.get(slot_idx).map(|(slot, _)| (slot_idx, slot)))
            })
            .expect("cannot get slot machine");

        let approx_median = self.tracker.approx_median();

        slot_machine.play(SearchContext { heuristic_ctx, from, slot_idx, solution, approx_median })
    }

    /// Updates estimations based on search feedback.
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        let from = &feedback.sample.transition.0;

        if let Some(slots) = self.slot_machines.get_mut(from) {
            slots[feedback.slot_idx].0.update(feedback);
        }

        self.tracker.observe_sample(generation, feedback.sample.clone())
    }

    /// Updates statistics about heuristic internal parameters.
    pub fn save_params(&mut self, generation: usize) {
        if !self.tracker.telemetry_enabled() {
            return;
        }

        self.slot_machines.iter().for_each(|(state, slots)| {
            slots.iter().map(|(slot, name)| (name.clone(), slot.get_params())).for_each(
                |(name, (alpha, beta, mu, v, n))| {
                    self.tracker.observe_params(
                        generation,
                        HeuristicSample { state: state.clone(), name, alpha, beta, mu, v, n },
                    );
                },
            )
        });
    }
}

impl<F, C, O, S> Display for DynamicSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.agent.tracker.telemetry_enabled() {
            return Ok(());
        }

        f.write_fmt(format_args!("TELEMETRY\n"))?;
        f.write_fmt(format_args!("search:\n"))?;
        f.write_fmt(format_args!("name,generation,reward,from,to,duration\n"))?;
        for (generation, sample) in self.agent.tracker.search_telemetry.iter() {
            f.write_fmt(format_args!(
                "{},{},{},{},{},{}\n",
                sample.name, generation, sample.reward, sample.transition.0, sample.transition.1, sample.duration
            ))?;
        }

        f.write_fmt(format_args!("heuristic:\n"))?;
        f.write_fmt(format_args!("generation,state,name,alpha,beta,mu,v,n\n"))?;
        for (generation, sample) in self.agent.tracker.heuristic_telemetry.iter() {
            f.write_fmt(format_args!(
                "{},{},{},{},{},{},{},{}\n",
                generation, sample.state, sample.name, sample.alpha, sample.beta, sample.mu, sample.v, sample.n
            ))?;
        }

        Ok(())
    }
}

fn compare_to_best<F, C, O, S>(heuristic_ctx: &C, solution: &S) -> Ordering
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    heuristic_ctx
        .ranked()
        .next()
        .map(|best_known| heuristic_ctx.objective().total_order(solution, best_known))
        .unwrap_or(Ordering::Less)
}

/// Estimates new solution discovery reward based on distance metric.
/// Returns a reward estimation in `[0, 6]` range. This range consists of:
/// - a initial distance improvement gives `[0, 2]`
/// - a best known improvement gives `[0, 2]` * BEST_DISCOVERY_REWARD_MULTIPLIER
fn estimate_distance_reward<F, C, O, S>(heuristic_ctx: &C, initial_solution: &S, new_solution: &S) -> f64
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    heuristic_ctx
        .ranked()
        .next()
        .map(|best_known| {
            const BEST_DISCOVERY_REWARD_MULTIPLIER: f64 = 2.;

            let objective = heuristic_ctx.objective();

            let distance_initial = get_relative_distance(objective, new_solution, initial_solution);
            let distance_best = get_relative_distance(objective, new_solution, best_known);

            // NOTE remap distances to range [0, 2]
            match (compare_floats(distance_initial, 0.), compare_floats(distance_best, 0.)) {
                (Ordering::Greater, Ordering::Greater) => {
                    (distance_initial + 1.) + (distance_best + 1.) * BEST_DISCOVERY_REWARD_MULTIPLIER
                }
                (Ordering::Greater, Ordering::Less) => distance_initial + 1.,
                _ => 0.,
            }
        })
        .unwrap_or(0.)
}

/// Estimates performance of used operation based on its duration and overall improvement statistics.
/// Returns a reward multiplier in `(~0.5, 3]` range.
fn estimate_reward_perf_multiplier<F, C, O, S>(
    search_ctx: &SearchContext<F, C, O, S>,
    duration: usize,
    has_improvement: bool,
) -> f64
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    let improvement_ratio = search_ctx.heuristic_ctx.statistics().improvement_1000_ratio;
    let approx_median = &search_ctx.approx_median;
    let median_ratio =
        approx_median.map_or(1., |median| if median == 0 { 1. } else { duration as f64 / median as f64 });

    let median_ratio = match median_ratio.clamp(0.5, 2.) {
        ratio if ratio < 0.75 => 1.5, // Allegro
        ratio if ratio < 1. => 1.25,  // Allegretto
        ratio if ratio > 1.5 => 0.75, // Andante
        _ => 1.,                      // Moderato
    };

    let improvement_ratio = match (improvement_ratio, has_improvement) {
        // stagnation: increase reward
        (ratio, true) if ratio < 0.05 => 2.,
        // fast convergence: decrease reward
        (ratio, true) if ratio > 0.150 => 0.75,
        // moderate convergence
        _ => 1.,
    };

    median_ratio * improvement_ratio
}

/// Returns a signed relative distance between two solutions. Sign specifies whether a solution is
/// better (positive) or worse (negative).
fn get_relative_distance<F, O, S>(objective: &O, a: &S, b: &S) -> f64
where
    F: HeuristicFitness,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    let order = objective.total_order(a, b);

    let sign = match order {
        Ordering::Less => 1.,
        Ordering::Greater => -1.,
        Ordering::Equal => return 0.,
    };

    let value = objective.distance(a, b);

    value * sign
}

/// Sample of search telemetry.
#[derive(Clone)]
struct SearchSample {
    name: String,
    duration: usize,
    reward: f64,
    transition: (SearchState, SearchState),
}

struct HeuristicSample {
    state: SearchState,
    name: String,
    alpha: f64,
    beta: f64,
    mu: f64,
    v: f64,
    n: usize,
}

/// Provides way to track heuristic's telemetry and duration median estimation.
struct HeuristicTracker {
    total_median: RemedianUsize,
    search_telemetry: Vec<(usize, SearchSample)>,
    heuristic_telemetry: Vec<(usize, HeuristicSample)>,
    is_experimental: bool,
}

impl HeuristicTracker {
    /// Returns true if telemetry is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.is_experimental
    }

    /// Returns median approximation.
    pub fn approx_median(&self) -> Option<usize> {
        self.total_median.approx_median()
    }

    /// Observes a current sample. Updates total duration median.
    pub fn observe_sample(&mut self, generation: usize, sample: SearchSample) {
        self.total_median.add_observation(sample.duration);
        if self.telemetry_enabled() {
            self.search_telemetry.push((generation, sample));
        }
    }

    pub fn observe_params(&mut self, generation: usize, sample: HeuristicSample) {
        if self.telemetry_enabled() {
            self.heuristic_telemetry.push((generation, sample));
        }
    }
}
