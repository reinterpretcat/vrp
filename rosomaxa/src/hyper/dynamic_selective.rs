#[cfg(test)]
#[path = "../../tests/unit/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use super::*;
use crate::Timer;
use crate::algorithms::rl::{BernoulliParams, BernoulliPosterior, SlotAction, SlotFeedback, SlotMachine};
use crate::utils::{DefaultDistributionSampler, ParallelismPolicy, parallel_collect, random_argmax};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::Hash;
use std::iter::once;
use std::sync::Arc;

type SearchOperator<C, O, S> = Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>;

/// Configures a search operator for dynamic selection.
pub struct HeuristicSearchOperatorConfig<C, O, S> {
    operator: SearchOperator<C, O, S>,
    name: String,
    initial_weight: Float,
    families: Vec<String>,
}

impl<C, O, S> HeuristicSearchOperatorConfig<C, O, S> {
    /// Creates an independent search operator configuration.
    pub fn new(operator: SearchOperator<C, O, S>, name: impl Into<String>, initial_weight: Float) -> Self {
        Self { operator, name: name.into(), initial_weight, families: Vec::new() }
    }

    /// Assigns families whose progress posteriors can weakly adjust related operators during selection.
    pub fn with_families<I, T>(mut self, families: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.families = families.into_iter().map(Into::into).fold(Vec::new(), |mut unique, family| {
            if !unique.contains(&family) {
                unique.push(family);
            }
            unique
        });
        self
    }

    /// Returns the operator name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// A collection of search operators configured for dynamic selection.
pub type HeuristicSearchOperators<C, O, S> = Vec<HeuristicSearchOperatorConfig<C, O, S>>;

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
        let feedbacks = parallel_collect(solutions, ParallelismPolicy::Coarse, |solution| {
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

struct SearchSlot<'a, C, O, S> {
    /// Learns incumbent improvements for best-known parents and parent improvements for diverse parents.
    progress: SlotMachine<SearchAction<'a, C, O, S>, DefaultDistributionSampler>,
    /// Learns whether an improved diverse parent is promoted past the incumbent.
    promotion: Option<BernoulliPosterior<DefaultDistributionSampler>>,
    name: String,
    /// Other slots grouped by each configured operator family.
    peer_groups: Vec<Vec<usize>>,
}

type SearchSlots<'a, C, O, S> = Vec<SearchSlot<'a, C, O, S>>;

impl<'a, C, O, S> SearchSlot<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'a,
{
    /// Samples an operator score from its learned progress and, for diverse parents, promotion probabilities.
    fn sample(&self, peer_progress: Option<Float>) -> Float {
        let progress = blend_progress_mean(self.progress.sample(), peer_progress);
        let promotion = self.promotion.as_ref().map_or(1., BernoulliPosterior::sample);

        progress * promotion
    }

    /// Records progress from the selected parent and a conditional promotion to the incumbent.
    fn update(&mut self, feedback: &SearchFeedback<S>) {
        self.progress.update(feedback);

        if feedback.sample.is_parent_improvement {
            if let Some(promotion) = self.promotion.as_mut() {
                promotion.update(feedback.sample.is_new_best);
            }
        }
    }
}

/// Bounds applied to relative operator weights before using them as successful-outcome priors.
const PRIOR_ALPHA_MIN: Float = 0.1;
const PRIOR_ALPHA_MAX: Float = 2.0;

/// Weakly regularizes sparse pair estimates while keeping the selected operator's evidence dominant.
const PEER_PROGRESS_WEIGHT: Float = 0.1;

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
        match &self.sample.transition.0 {
            SearchState::BestKnown => self.sample.is_new_best,
            SearchState::Diverse => self.sample.is_parent_improvement,
        }
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

        let objective = context.heuristic_ctx.objective();
        let is_new_best = context
            .best_known
            .is_some_and(|best_known| objective.total_order(&new_solution, best_known) == Ordering::Less);
        let is_parent_improvement = match &context.from {
            // A best-known parent ties the incumbent, while beating the incumbent from a diverse parent also
            // implies beating that parent. Only the remaining case needs another objective comparison.
            SearchState::BestKnown => is_new_best,
            SearchState::Diverse => {
                is_new_best || objective.total_order(&new_solution, context.solution) == Ordering::Less
            }
        };
        let to = if is_new_best { SearchState::BestKnown } else { SearchState::Diverse };
        let transition = (context.from, to);

        let sample = SearchSample { duration, transition, is_parent_improvement, is_new_best };

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
    slot_machines: HashMap<SearchState, SearchSlots<'a, C, O, S>>,
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
        let total_weight: Float = search_operators.iter().map(|config| config.initial_weight).sum();
        let count = search_operators.len() as Float;
        let avg_weight = if count > 0.0 && total_weight > f64::EPSILON { total_weight / count } else { 1.0 };

        // Factory function to create slot configurations for each state.
        // Uses domain knowledge (initial weights) as priors - important because:
        // 1. We have many operators (cold start problem)
        // 2. Limited search time may not be enough to learn from scratch
        // 3. Weights encode expert knowledge about operator effectiveness
        let peer_groups = create_peer_groups(&search_operators);
        let create_slots = |with_promotion: bool| {
            search_operators
                .iter()
                .zip(peer_groups.iter())
                .map(|(config, peer_groups)| {
                    let prior_alpha = get_prior_alpha(config.initial_weight, avg_weight);
                    SearchSlot {
                        progress: SlotMachine::new(
                            prior_alpha,
                            SearchAction { operator: config.operator.clone() },
                            DefaultDistributionSampler::new(environment.random.clone()),
                        ),
                        promotion: with_promotion.then(|| {
                            BernoulliPosterior::new(1., DefaultDistributionSampler::new(environment.random.clone()))
                        }),
                        name: config.name.clone(),
                        peer_groups: peer_groups.clone(),
                    }
                })
                .collect::<Vec<_>>()
        };

        // Initialize separate states with identical priors but independent learning.
        let slot_machines = once((SearchState::BestKnown, create_slots(false)))
            .chain(once((SearchState::Diverse, create_slots(true))))
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

        self.slot_machines.values_mut().flat_map(|slots| slots.iter_mut()).for_each(|slot| {
            slot.progress.reset();
            if let Some(promotion) = slot.promotion.as_mut() {
                promotion.reset();
            }
        });

        (self.stagnation_reset_interval, self.next_stagnation_reset) =
            advance_stagnation_reset(statistics.generation, self.stagnation_reset_interval);
    }

    /// Picks the relevant search operator using contextual Thompson sampling and runs the search.
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

        let samples = slots.iter().enumerate().map(|(slot_idx, slot)| {
            let peer_progress = get_peer_progress(slots, slot_idx);
            slot.sample(peer_progress)
        });
        let slot_idx = random_argmax(samples, self.random.as_ref()).unwrap_or(0);
        let slot_machine = &slots[slot_idx].progress;

        // Execute with full context information.
        slot_machine.play(SearchContext { heuristic_ctx, best_known, from, slot_idx, solution })
    }

    /// Updates the selected slot with the outcomes relevant to its parent state.
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        if feedback.sample.transition.1 == SearchState::BestKnown {
            self.stagnation_reset_interval = STAGNATION_WINDOW;
            self.next_stagnation_reset = generation.saturating_add(STAGNATION_WINDOW);
        }

        let from = &feedback.sample.transition.0;
        let slots = self.slot_machines.get_mut(from).expect("cannot get slot machines");
        let slot = &mut slots[feedback.slot_idx];
        slot.update(feedback);

        // Track telemetry.
        self.tracker.observe_sample(generation, &slot.name, &feedback.sample);
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
                slots.iter().enumerate().map(|(slot_idx, slot)| {
                    let progress = slot.progress.get_params();
                    let selection = blend_progress(progress, get_peer_progress(slots, slot_idx));
                    let (effective_mean, effective_variance, promotion_mean) =
                        slot.promotion.as_ref().map_or((selection.mean, selection.variance, 1.), |promotion| {
                            let promotion = promotion.params();
                            (selection.mean * promotion.mean, product_variance(&selection, &promotion), promotion.mean)
                        });
                    let summary = self.tracker.get_summary(state, &slot.name);
                    HeuristicSample {
                        state: state.clone(),
                        name: slot.name.clone(),
                        progress_alpha: progress.alpha,
                        progress_beta: progress.beta,
                        effective_mean,
                        effective_variance,
                        calls: progress.observations,
                        incumbent_improvements: summary.incumbent_improvements,
                        duration: summary.duration,
                        progress_mean: progress.mean,
                        promotion_mean,
                        parent_improvements: summary.parent_improvements,
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

fn create_peer_groups<C, O, S>(configs: &[HeuristicSearchOperatorConfig<C, O, S>]) -> Vec<Vec<Vec<usize>>> {
    configs
        .iter()
        .enumerate()
        .map(|(slot_idx, config)| {
            config
                .families
                .iter()
                .filter_map(|family| {
                    let peers = configs
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, config)| {
                            (idx != slot_idx && config.families.contains(family)).then_some(idx)
                        })
                        .collect::<Vec<_>>();

                    (!peers.is_empty()).then_some(peers)
                })
                .collect()
        })
        .collect()
}

fn get_peer_progress<C, O, S>(slots: &[SearchSlot<'_, C, O, S>], slot_idx: usize) -> Option<Float>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let groups = &slots[slot_idx].peer_groups;
    (!groups.is_empty()).then(|| {
        // Average each family first so a component with more combinations does not get more influence.
        groups
            .iter()
            .map(|group| {
                group.iter().map(|&idx| slots[idx].progress.get_params().mean).sum::<Float>() / group.len() as Float
            })
            .sum::<Float>()
            / groups.len() as Float
    })
}

fn blend_progress(mut progress: BernoulliParams, peer_progress: Option<Float>) -> BernoulliParams {
    if let Some(peer) = peer_progress {
        progress.mean = blend_progress_mean(progress.mean, Some(peer));
        progress.variance *= (1. - PEER_PROGRESS_WEIGHT).powi(2);
    }
    progress
}

fn blend_progress_mean(progress: Float, peer_progress: Option<Float>) -> Float {
    peer_progress.map_or(progress, |peer| progress * (1. - PEER_PROGRESS_WEIGHT) + peer * PEER_PROGRESS_WEIGHT)
}

fn get_duration_micros(duration: std::time::Duration) -> usize {
    (duration.as_micros().min(usize::MAX as u128) as usize).max(1)
}

fn product_variance(left: &BernoulliParams, right: &BernoulliParams) -> Float {
    left.variance * right.variance + left.variance * right.mean.powi(2) + right.variance * left.mean.powi(2)
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
    incumbent_improvements: usize,
    duration: u64,
    parent_improvements: usize,
}

impl HeuristicSummary {
    fn observe(&mut self, sample: &SearchSample) {
        self.incumbent_improvements += usize::from(sample.is_new_best);
        self.duration = self.duration.saturating_add(sample.duration as u64);
        self.parent_improvements += usize::from(sample.is_parent_improvement);
    }
}

/// A sample of search telemetry.
#[derive(Clone)]
struct SearchSample {
    duration: usize,
    transition: (SearchState, SearchState),
    is_parent_improvement: bool,
    is_new_best: bool,
}

/// A sample of heuristic parameters telemetry.
struct HeuristicSample {
    state: SearchState,
    name: String,
    progress_alpha: Float,
    progress_beta: Float,
    effective_mean: Float,
    effective_variance: Float,
    calls: usize,
    incumbent_improvements: usize,
    duration: u64,
    progress_mean: Float,
    promotion_mean: Float,
    parent_improvements: usize,
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
        f.write_fmt(format_args!("heuristic:\n"))?;
        f.write_fmt(format_args!(
            "generation,state,name,progress_alpha,progress_beta,effective_mu,effective_v,calls,incumbent_improvements,duration_us,progress_mu,promotion_mu,parent_improvements\n"
        ))?;

        let final_generation = self.agent.tracker.last_generation;
        let final_params = self.agent.get_params();
        for (generation, samples) in
            self.agent.tracker.heuristic_telemetry.iter().filter(|(generation, _)| *generation != final_generation)
        {
            for sample in samples {
                f.write_fmt(format_args!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                    generation,
                    sample.state,
                    sample.name,
                    sample.progress_alpha,
                    sample.progress_beta,
                    sample.effective_mean,
                    sample.effective_variance,
                    sample.calls,
                    sample.incumbent_improvements,
                    sample.duration,
                    sample.progress_mean,
                    sample.promotion_mean,
                    sample.parent_improvements
                ))?;
            }
        }
        for sample in final_params {
            f.write_fmt(format_args!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                final_generation,
                sample.state,
                sample.name,
                sample.progress_alpha,
                sample.progress_beta,
                sample.effective_mean,
                sample.effective_variance,
                sample.calls,
                sample.incumbent_improvements,
                sample.duration,
                sample.progress_mean,
                sample.promotion_mean,
                sample.parent_improvements
            ))?;
        }

        Ok(())
    }
}
