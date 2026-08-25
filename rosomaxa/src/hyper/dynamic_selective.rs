#[cfg(test)]
#[path = "../../tests/unit/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use super::*;
use crate::Timer;
use crate::algorithms::rl::{SlotAction, SlotFeedback, SlotMachine};
use crate::utils::{DefaultDistributionSampler, ParallelismScope, random_argmax};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::Hash;
use std::iter::once;
use std::sync::Arc;

/// A collection of heuristic search operators with their name and initial weight.
pub type HeuristicSearchOperators<C, O, S> =
    Vec<(Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>, String, Float)>;

/// An experimental dynamic selective hyper heuristic which selects inner heuristics
/// based on how they work during the search. The selection process is modeled using reinforcement
/// learning techniques.
pub struct DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    agent: SearchAgent<'static, C, O, S>,
    diversify_operators: HeuristicDiversifyOperators<C, O, S>,
    intensify_operators: HeuristicIntensifyOperators<C, O, S>,
}

impl<C, O, S> HyperHeuristic for DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'static,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let generation = heuristic_ctx.statistics().generation;
        self.agent.reset_if_stagnant(heuristic_ctx.statistics());
        let feedback = self.agent.search(heuristic_ctx, solution);

        self.agent.update(generation, &feedback);

        feedback.solution.into_iter().collect()
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        self.agent.reset_if_stagnant(heuristic_ctx.statistics());
        // Population is unchanged while the batch runs, so all searches can use the same best solution.
        let best_known = heuristic_ctx.ranked().next();
        let feedbacks = parallel_into_collect(solutions, ParallelismScope::Coarse, |solution| {
            self.agent.search_with_best(heuristic_ctx, solution, best_known)
        });

        let generation = heuristic_ctx.statistics().generation;
        feedbacks.iter().for_each(|feedback| {
            self.agent.update(generation, feedback);
        });

        self.agent.save_params(generation);

        feedbacks.into_iter().filter_map(|feedback| feedback.solution).collect()
    }

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        diversify_solution(heuristic_ctx, solution, self.diversify_operators.as_slice())
    }

    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        diversify_solutions(heuristic_ctx, solutions, self.diversify_operators.as_slice())
    }

    fn intensify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        intensify_solution(heuristic_ctx, solution, self.intensify_operators.as_slice())
    }

    fn intensify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        intensify_solutions(heuristic_ctx, solutions, self.intensify_operators.as_slice())
    }
}

impl<C, O, S> DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'static,
{
    /// Creates a new instance of `DynamicSelective` heuristic.
    pub fn new(search_operators: HeuristicSearchOperators<C, O, S>, environment: &Environment) -> Self {
        Self {
            agent: SearchAgent::new(search_operators, environment),
            diversify_operators: Vec::new(),
            intensify_operators: Vec::new(),
        }
    }

    /// Adds operators which diversify search during exploration.
    pub fn with_diversify_operators(mut self, operators: HeuristicDiversifyOperators<C, O, S>) -> Self {
        self.diversify_operators = operators;
        self
    }

    /// Adds operators which intensify search during exploitation.
    pub fn with_intensify_operators(mut self, operators: HeuristicIntensifyOperators<C, O, S>) -> Self {
        self.intensify_operators = operators;
        self
    }
}

/// Type alias for slot machines used in Thompson sampling.
pub type SlotMachines<'a, C, O, S> = Vec<(SlotMachine<SearchAction<'a, C, O, S>, DefaultDistributionSampler>, String)>;

/// Bounds applied to relative operator weights before using them as successful-outcome priors.
const PRIOR_ALPHA_MIN: Float = 0.1;
const PRIOR_ALPHA_MAX: Float = 2.0;

/// Restarts learned operator posteriors after this many generations without improvement.
const STAGNATION_WINDOW: usize = 1000;

/// Search state for Thompson sampling.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum SearchState {
    /// Best known solution state.
    BestKnown,
    /// Diverse solution state.
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

/// Search feedback result for Thompson sampling.
pub struct SearchFeedback<S> {
    sample: SearchSample,
    slot_idx: usize,
    solution: Option<S>,
}

impl<S> SlotFeedback for SearchFeedback<S> {
    fn is_success(&self) -> bool {
        self.sample.reward > 0.
    }
}

/// Search action wrapper for Thompson sampling.
pub struct SearchAction<'a, C, O, S> {
    operator: Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync + 'a>,
}

impl<C, O, S> Clone for SearchAction<'_, C, O, S> {
    fn clone(&self) -> Self {
        Self { operator: self.operator.clone() }
    }
}

impl<'a, C, O, S> SlotAction for SearchAction<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'a,
{
    type Context = SearchContext<'a, C, O, S>;
    type Feedback = SearchFeedback<S>;

    fn take(&self, context: Self::Context) -> Self::Feedback {
        let (new_solution, duration) =
            Timer::measure_duration(|| self.operator.search(context.heuristic_ctx, context.solution));

        let duration = get_duration_micros(duration);

        // Only a strict incumbent improvement is a success. Improving a weak parent is useful search
        // movement, but rewarding it here lets frequent local moves starve rare basin-changing operators.
        let is_new_best = context.best_known.is_some_and(|best_known| {
            context.heuristic_ctx.objective().total_order(&new_solution, best_known) == Ordering::Less
        });
        let reward = if is_new_best { 1. } else { 0. };

        let to = if is_new_best { SearchState::BestKnown } else { SearchState::Diverse };
        let transition = (context.from, to);

        let sample = SearchSample { duration, reward, transition };

        SearchFeedback { sample, slot_idx: context.slot_idx, solution: Some(new_solution) }
    }
}

/// Search context for Thompson sampling.
pub struct SearchContext<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx: &'a C,
    best_known: Option<&'a S>,
    from: SearchState,
    slot_idx: usize,
    solution: &'a S,
}

struct SearchAgent<'a, C, O, S> {
    /// Separate learning contexts for different search phases (BestKnown vs Diverse).
    slot_machines: HashMap<SearchState, SlotMachines<'a, C, O, S>>,
    /// Tracks experimental operator statistics.
    tracker: HeuristicTracker,
    /// Random number generator for Thompson sampling selection.
    random: Arc<dyn Random>,
    /// Current delay between posterior resets during stagnation.
    stagnation_reset_interval: usize,
    /// Generation at which the next stagnant posterior can be reset.
    next_stagnation_reset: usize,
}

impl<'a, C, O, S> SearchAgent<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'a,
{
    pub fn new(search_operators: HeuristicSearchOperators<C, O, S>, environment: &Environment) -> Self {
        // Normalize expert weights so an average operator starts with one successful pseudo-observation.
        let total_weight: Float = search_operators.iter().map(|(_, _, w)| *w).sum();
        let count = search_operators.len() as Float;
        let avg_weight = if count > 0.0 && total_weight > f64::EPSILON { total_weight / count } else { 1.0 };

        // Factory function to create slot configurations for each state.
        // Uses domain knowledge (initial weights) as priors - important because:
        // 1. We have many operators (cold start problem)
        // 2. Limited search time may not be enough to learn from scratch
        // 3. Weights encode expert knowledge about operator effectiveness
        let create_slots = || {
            search_operators
                .iter()
                .map(|(operator, name, initial_weight)| {
                    let prior_alpha = get_prior_alpha(*initial_weight, avg_weight);
                    (
                        SlotMachine::new(
                            prior_alpha,
                            SearchAction { operator: operator.clone() },
                            DefaultDistributionSampler::new(environment.random.clone()),
                        ),
                        name.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };

        // Initialize separate states with identical priors but independent learning.
        let slot_machines = once((SearchState::BestKnown, create_slots()))
            .chain(once((SearchState::Diverse, create_slots())))
            .collect();

        Self {
            slot_machines,
            tracker: HeuristicTracker::new(environment.is_experimental),
            random: environment.random.clone(),
            stagnation_reset_interval: STAGNATION_WINDOW,
            next_stagnation_reset: STAGNATION_WINDOW,
        }
    }

    fn reset_if_stagnant(&mut self, statistics: &HeuristicStatistics) {
        if !should_reset(statistics, self.next_stagnation_reset) {
            return;
        }

        self.slot_machines.values_mut().flat_map(|slots| slots.iter_mut()).for_each(|(slot, _)| slot.reset());

        (self.stagnation_reset_interval, self.next_stagnation_reset) =
            advance_stagnation_reset(statistics.generation, self.stagnation_reset_interval);
    }

    /// Picks the relevant search operator using pure Thompson Sampling and runs the search.
    pub fn search(&self, heuristic_ctx: &C, solution: &S) -> SearchFeedback<S> {
        let best_known = heuristic_ctx.ranked().next();
        self.search_with_best(heuristic_ctx, solution, best_known)
    }

    fn search_with_best(&self, heuristic_ctx: &C, solution: &S, best_known: Option<&S>) -> SearchFeedback<S> {
        // Determine search context - critical for operator selection.
        let from = if matches!(compare_to_best(heuristic_ctx.objective(), best_known, solution), Ordering::Equal) {
            SearchState::BestKnown
        } else {
            SearchState::Diverse
        };

        // Get contextually appropriate slot machines.
        let slots = self.slot_machines.get(&from).expect("cannot get slot machines");

        // Sample each arm, pick argmax with random tie-break.
        let samples = slots.iter().map(|(slot, _)| slot.sample());
        let slot_idx = random_argmax(samples, self.random.as_ref()).unwrap_or(0);
        let slot_machine = &slots[slot_idx].0;

        // Execute with full context information.
        slot_machine.play(SearchContext { heuristic_ctx, best_known, from, slot_idx, solution })
    }

    /// Updates the selected slot with its incumbent-improvement outcome.
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        if feedback.sample.transition.1 == SearchState::BestKnown {
            self.stagnation_reset_interval = STAGNATION_WINDOW;
            self.next_stagnation_reset = generation.saturating_add(STAGNATION_WINDOW);
        }

        let from = &feedback.sample.transition.0;
        let slots = self.slot_machines.get_mut(from).expect("cannot get slot machines");
        let (slot_machine, name) = &mut slots[feedback.slot_idx];
        slot_machine.update(feedback);

        // Track telemetry.
        self.tracker.observe_sample(generation, name, &feedback.sample);
    }

    /// Updates statistics about heuristic internal parameters.
    pub fn save_params(&mut self, generation: usize) {
        if !self.tracker.should_record_params(generation) {
            return;
        }

        self.tracker.observe_params(generation, self.get_params());
    }

    fn get_params(&self) -> Vec<HeuristicSample> {
        self.slot_machines
            .iter()
            .flat_map(|(state, slots)| {
                slots.iter().map(|(slot, name)| {
                    let (alpha, beta, mu, v, n) = slot.get_params();
                    let summary = self.tracker.get_summary(state, name);
                    HeuristicSample {
                        state: state.clone(),
                        name: name.clone(),
                        alpha,
                        beta,
                        mu,
                        v,
                        n,
                        successes: summary.successes,
                        duration: summary.duration,
                    }
                })
            })
            .collect()
    }
}

fn should_reset(statistics: &HeuristicStatistics, next_reset: usize) -> bool {
    statistics.generation >= next_reset && statistics.improvement_1000_ratio == 0.
}

fn advance_stagnation_reset(generation: usize, interval: usize) -> (usize, usize) {
    let interval = interval.saturating_add(STAGNATION_WINDOW);
    (interval, generation.saturating_add(interval))
}

/// Maps an operator's relative expert weight to its successful-outcome prior.
fn get_prior_alpha(initial_weight: Float, avg_weight: Float) -> Float {
    (initial_weight / avg_weight).clamp(PRIOR_ALPHA_MIN, PRIOR_ALPHA_MAX)
}

fn get_duration_micros(duration: std::time::Duration) -> usize {
    (duration.as_micros().min(usize::MAX as u128) as usize).max(1)
}

fn compare_to_best<O, S>(objective: &O, best_known: Option<&S>, solution: &S) -> Ordering
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    best_known.map(|best_known| objective.total_order(solution, best_known)).unwrap_or(Ordering::Less)
}

/// Diagnostic tracker for Thompson sampling analysis.
struct HeuristicTracker {
    heuristic_telemetry: Vec<(usize, Vec<HeuristicSample>)>,
    summaries: HashMap<SearchState, HashMap<String, HeuristicSummary>>,
    recording_interval: usize,
    last_generation: usize,
    is_experimental: bool,
}

impl HeuristicTracker {
    /// Creates a new tracker with diagnostic configuration.
    pub fn new(is_experimental: bool) -> Self {
        Self {
            heuristic_telemetry: Default::default(),
            summaries: Default::default(),
            recording_interval: 1,
            last_generation: 0,
            is_experimental,
        }
    }

    /// Returns true if telemetry is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.is_experimental
    }

    /// Observes the current sample.
    pub fn observe_sample(&mut self, generation: usize, name: &str, sample: &SearchSample) {
        if self.telemetry_enabled() {
            self.last_generation = generation;
            let state = &sample.transition.0;
            if let Some(summary) = self.summaries.get_mut(state).and_then(|summaries| summaries.get_mut(name)) {
                summary.observe(sample);
            } else {
                let mut summary = HeuristicSummary::default();
                summary.observe(sample);
                self.summaries.entry(state.clone()).or_default().insert(name.to_string(), summary);
            }
        }
    }

    /// Returns exact cumulative telemetry for a state-specific operator.
    fn get_summary(&self, state: &SearchState, name: &str) -> HeuristicSummary {
        self.summaries.get(state).and_then(|summaries| summaries.get(name)).copied().unwrap_or_default()
    }

    fn should_record_params(&self, generation: usize) -> bool {
        self.telemetry_enabled() && generation.is_multiple_of(self.recording_interval)
    }

    /// Retains complete posterior banks at an adaptive interval.
    pub fn observe_params(&mut self, generation: usize, samples: Vec<HeuristicSample>) {
        const MAX_RETAINED_PARAMS: usize = 20_000;

        if !self.telemetry_enabled() || samples.is_empty() {
            return;
        }

        let max_snapshots = (MAX_RETAINED_PARAMS / samples.len()).max(2);
        self.heuristic_telemetry.push((generation, samples));
        compact_params(&mut self.heuristic_telemetry, &mut self.recording_interval, max_snapshots);
    }
}

fn compact_params(
    telemetry: &mut Vec<(usize, Vec<HeuristicSample>)>,
    recording_interval: &mut usize,
    max_snapshots: usize,
) {
    if telemetry.len() > max_snapshots {
        *recording_interval = recording_interval.saturating_mul(2).max(1);
        telemetry.retain(|(generation, _)| generation.is_multiple_of(*recording_interval));
    }
}

#[derive(Clone, Copy, Default)]
struct HeuristicSummary {
    successes: usize,
    duration: u64,
}

impl HeuristicSummary {
    fn observe(&mut self, sample: &SearchSample) {
        self.successes += usize::from(sample.reward > 0.);
        self.duration = self.duration.saturating_add(sample.duration as u64);
    }
}

/// A sample of search telemetry.
#[derive(Clone)]
struct SearchSample {
    duration: usize,
    reward: Float,
    transition: (SearchState, SearchState),
}

/// A sample of heuristic parameters telemetry.
struct HeuristicSample {
    state: SearchState,
    name: String,
    alpha: Float,
    beta: Float,
    mu: Float,
    v: Float,
    n: usize,
    successes: usize,
    duration: u64,
}

impl<C, O, S> Display for DynamicSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.agent.tracker.telemetry_enabled() {
            return Ok(());
        }

        f.write_fmt(format_args!("TELEMETRY\n"))?;
        f.write_fmt(format_args!("search:\n"))?;
        f.write_fmt(format_args!("name,generation,reward,from,to,duration_us\n"))?;

        f.write_fmt(format_args!("heuristic:\n"))?;
        f.write_fmt(format_args!("generation,state,name,alpha,beta,mu,v,n,successes,duration_us\n"))?;

        let final_generation = self.agent.tracker.last_generation;
        let final_params = self.agent.get_params();
        for (generation, samples) in
            self.agent.tracker.heuristic_telemetry.iter().filter(|(generation, _)| *generation != final_generation)
        {
            for sample in samples {
                f.write_fmt(format_args!(
                    "{},{},{},{},{},{},{},{},{},{}\n",
                    generation,
                    sample.state,
                    sample.name,
                    sample.alpha,
                    sample.beta,
                    sample.mu,
                    sample.v,
                    sample.n,
                    sample.successes,
                    sample.duration
                ))?;
            }
        }
        for sample in final_params {
            f.write_fmt(format_args!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                final_generation,
                sample.state,
                sample.name,
                sample.alpha,
                sample.beta,
                sample.mu,
                sample.v,
                sample.n,
                sample.successes,
                sample.duration
            ))?;
        }

        Ok(())
    }
}
