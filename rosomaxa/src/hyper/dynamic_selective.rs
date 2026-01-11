#[cfg(test)]
#[path = "../../tests/unit/hyper/dynamic_selective_test.rs"]
mod dynamic_selective_test;

use super::*;
use crate::Timer;
use crate::algorithms::math::RemedianUsize;
use crate::algorithms::rl::{SlotAction, SlotFeedback, SlotMachine};
use crate::utils::{DefaultDistributionSampler, random_argmax};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::Hash;
use std::iter::once;
use std::sync::Arc;

/// A collection of heuristic search operators with their name and initial weight.
pub type HeuristicSearchOperators<C, O, S> =
    Vec<(Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>, String, Float)>;

/// A collection of heuristic diversify operators.
pub type HeuristicDiversifyOperators<C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>>;

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
        let feedback = self.agent.search(heuristic_ctx, solution);

        self.agent.update(generation, &feedback);

        feedback.solution.into_iter().collect()
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

        feedbacks.into_iter().filter_map(|feedback| feedback.solution).collect()
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
    C: HeuristicContext<Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'static,
{
    /// Creates a new instance of `DynamicSelective` heuristic.
    pub fn new(
        search_operators: HeuristicSearchOperators<C, O, S>,
        diversify_operators: HeuristicDiversifyOperators<C, O, S>,
        environment: &Environment,
    ) -> Self {
        Self { agent: SearchAgent::new(search_operators, environment), diversify_operators }
    }
}

/// Type alias for slot machines used in Thompson sampling.
pub type SlotMachines<'a, C, O, S> = Vec<(SlotMachine<SearchAction<'a, C, O, S>, DefaultDistributionSampler>, String)>;

/// Centralized "Physics" for the Reward System.
///
/// This struct defines the constants that control how rewards are calculated,
/// scaled, and clamped. Changing values here automatically propagates to
/// reward estimation and signal normalization logic.
struct SearchRewards;

impl SearchRewards {
    /// The base value for a standard improvement (Local Search success).
    /// Used as the atomic unit for signal normalization.
    pub const BASE_REWARD: Float = 2.0;

    /// The offset added to relative distance to ensure positive rewards.
    /// Logic: `(distance + OFFSET)`.
    pub const DISTANCE_OFFSET: Float = 1.0;

    /// Multiplier applied when a solution improves the Global Best Known.
    /// This is the "Jackpot" factor.
    pub const GLOBAL_BEST_MULTIPLIER: Float = 2.5;

    /// Multiplier applied when a solution is diverse but not a global improvement.
    /// Keeps "Diverse" operators alive but with low signal.
    pub const DIVERSE_MULTIPLIER: Float = 0.05;

    /// The maximum percentage (+/-) that execution duration can affect the reward.
    /// e.g., 0.2 means performance can scale reward by [0.8, 1.2].
    pub const PERF_TOLERANCE: Float = 0.2;

    /// The "Cheap Failure"
    /// The penalty applied when an operator produces no improvement.
    /// A small negative value ensures that "doing nothing" is worse than "finding diverse solutions".
    /// This forces the Slot Machine to eventually lower the confidence of stagnating operators.
    pub const PENALTY_MIN: Float = -0.01;

    /// The "Expensive Failure"
    /// The penalty applied when an operator produces a negative outcome.
    /// A larger negative value ensures that failures are strongly discouraged.
    pub const PENALTY_MAX: Float = -0.1;

    /// Calculates the ratio between the "Jackpot" and the "Normal" signal.
    /// Used by the Agent to determine how many times larger than the average
    /// a signal must be to be considered an "Outlier" that needs clamping.
    pub fn signal_clamp_ratio(max_dist: Float) -> Float {
        // Recalculate based on new constants
        let base_term = max_dist + Self::DISTANCE_OFFSET;
        let global_term = base_term * Self::GLOBAL_BEST_MULTIPLIER;
        let max_theoretical = (base_term + global_term) * (1.0 + Self::PERF_TOLERANCE);

        let typical_reward = (1.0 + Self::DISTANCE_OFFSET) * Self::BASE_REWARD;

        max_theoretical / typical_reward
    }
}

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
    fn reward(&self) -> Float {
        self.sample.reward
    }
}

/// Search action wrapper for Thompson sampling.
pub struct SearchAction<'a, C, O, S> {
    operator: Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync + 'a>,
    operator_name: String,
}

impl<C, O, S> Clone for SearchAction<'_, C, O, S> {
    fn clone(&self) -> Self {
        Self { operator: self.operator.clone(), operator_name: self.operator_name.clone() }
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

        let duration = duration.as_millis() as usize;

        // 1. Analyze result.
        let distance_score = estimate_distance_reward(context.heuristic_ctx, context.solution, &new_solution);

        // 2. Calculate final reward / penalty.
        let reward = match distance_score {
            // Success: Apply performance multiplier (bonus/penalty within +/- 20%).
            Some(base_score) => {
                let mult = estimate_reward_perf_multiplier(&context, duration);
                base_score * mult
            }
            // Failure: Apply time-weighted penalty.
            None => {
                // Get median (defensive default to duration itself to avoid div/0).
                let median = context.approx_median.unwrap_or(duration).max(1) as Float;
                let ratio = duration as Float / median;

                // Logic:
                // Ratio 0.2 (Fast)  -> -0.01 * 0.2 = -0.002 (Too small, clamp to MIN) -> -0.01
                // Ratio 1.0 (Avg)   -> -0.01 * 1.0 = -0.01
                // Ratio 5.0 (Slow)  -> -0.01 * 5.0 = -0.05
                // Ratio 10.0 (Slower) -> -0.01 * 10.0 = -0.1 (At MAX boundary)
                // Ratio 20.0 (Very Slow) -> -0.01 * 20.0 = -0.2 (Clamp to MAX) -> -0.1

                // Base penalty unit: PENALTY_MIN (-0.01) per "Median Unit of Time".
                let raw_penalty = SearchRewards::PENALTY_MIN * ratio;

                // Clamp to the range [PENALTY_MAX, PENALTY_MIN] = [-0.1, -0.01].
                // Note: min/max semantics with negative numbers:
                // - max(PENALTY_MAX) ensures we don't go below -0.1 (more negative).
                // - min(PENALTY_MIN) ensures we don't go above -0.01 (less negative).
                raw_penalty.max(SearchRewards::PENALTY_MAX).min(SearchRewards::PENALTY_MIN)
            }
        };

        let is_new_best = compare_to_best(context.heuristic_ctx, &new_solution) == Ordering::Less;
        let to = if is_new_best { SearchState::BestKnown } else { SearchState::Diverse };
        let transition = (context.from, to);

        let sample = SearchSample { name: self.operator_name.clone(), duration, reward, transition };

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
    from: SearchState,
    slot_idx: usize,
    solution: &'a S,
    approx_median: Option<usize>,
}

struct SearchAgent<'a, C, O, S> {
    // Separate learning contexts for different search phases
    slot_machines: HashMap<SearchState, SlotMachines<'a, C, O, S>>,
    // Shared scale invariance (universal physics)
    signal_stats: SignalStats,
    tracker: HeuristicTracker,
    random: Arc<dyn Random>,
}

impl<'a, C, O, S> SearchAgent<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'a,
{
    pub fn new(search_operators: HeuristicSearchOperators<C, O, S>, environment: &Environment) -> Self {
        // Calculate the Mean Weight
        // We want the "Average Operator" to have a prior mu = 1.0.
        // This aligns with SignalStats which normalizes the average reward to 1.0.
        let total_weight: Float = search_operators.iter().map(|(_, _, w)| *w).sum();
        let count = search_operators.len() as Float;

        let avg_weight = if count > 0.0 { total_weight / count } else { 1.0 };

        // Avoid division by zero if weights are weird
        let target_mean = SearchRewards::BASE_REWARD;
        let scale = if avg_weight > f64::EPSILON { target_mean / avg_weight } else { target_mean };

        // Factory function to create identical slot configurations for each state
        let create_slots = || {
            search_operators
                .iter()
                .map(|(operator, name, initial_weight)| {
                    let prior_mean = initial_weight * scale;
                    (
                        SlotMachine::new(
                            prior_mean,
                            SearchAction { operator: operator.clone(), operator_name: name.to_string() },
                            DefaultDistributionSampler::new(environment.random.clone()),
                        ),
                        name.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };

        // Initialize separate states with identical priors but independent learning
        let slot_machines = once((SearchState::BestKnown, create_slots()))
            .chain(once((SearchState::Diverse, create_slots())))
            .collect();

        Self {
            slot_machines,
            signal_stats: SignalStats::new(),
            tracker: HeuristicTracker::new(environment.is_experimental),
            random: environment.random.clone(),
        }
    }

    /// Picks the relevant search operator based on learnings and runs the search.
    pub fn search(&self, heuristic_ctx: &C, solution: &S) -> SearchFeedback<S> {
        // Determine search context - critical for operator selection.
        let from = if matches!(compare_to_best(heuristic_ctx, solution), Ordering::Equal) {
            SearchState::BestKnown
        } else {
            SearchState::Diverse
        };

        // Get contextually appropriate slot machines.
        let (slot_idx, slot_machine) = self
            .slot_machines
            .get(&from)
            .and_then(|slots| {
                random_argmax(slots.iter().map(|(slot, _)| slot.sample()), self.random.as_ref())
                    .and_then(|slot_idx| slots.get(slot_idx).map(|(slot, _)| (slot_idx, slot)))
            })
            .expect("cannot get slot machine");

        let approx_median = self.tracker.approx_median();

        // Execute with full context information.
        slot_machine.play(SearchContext { heuristic_ctx, from, slot_idx, solution, approx_median })
    }

    /// Updates estimations based on search feedback with protected signal baseline.
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        let max_dist = feedback.solution.as_ref().map_or(1., |s| s.fitness().count() as Float);
        let raw_reward = feedback.sample.reward;
        let current_scale = self.signal_stats.scale();

        // 1. Update ruler.
        // We only update the "Scale of Success" based on positive outcomes.
        // We do NOT shrink the ruler just because we failed.
        if raw_reward > f64::EPSILON {
            let clamp_limit = current_scale * SearchRewards::signal_clamp_ratio(max_dist);
            self.signal_stats.update(raw_reward.min(clamp_limit));
        }

        // 2. Normalize.
        let normalized_reward =
            if raw_reward.abs() > f64::EPSILON { raw_reward / self.signal_stats.scale() } else { 0.0 };
        let normalized_feedback = SearchFeedback {
            sample: SearchSample { reward: normalized_reward, ..feedback.sample.clone() },
            slot_idx: feedback.slot_idx,
            solution: None,
        };

        // 3. Update contextual slot machine.
        let from = &feedback.sample.transition.0;
        let slots = self.slot_machines.get_mut(from).expect("cannot get slot machines");
        slots[feedback.slot_idx].0.update(&normalized_feedback);

        // 4. Observe telemetry.
        self.tracker.observe_sample(generation, feedback.sample.clone());
    }

    /// Updates statistics about heuristic internal parameters.
    pub fn save_params(&mut self, generation: usize) {
        if !self.tracker.telemetry_enabled() {
            return;
        }

        self.slot_machines.iter().for_each(|(state, slots)| {
            slots.iter().for_each(|(slot, name)| {
                let (alpha, beta, mu, v, n) = slot.get_params();
                self.tracker.observe_params(
                    generation,
                    HeuristicSample { state: state.clone(), name: name.clone(), alpha, beta, mu, v, n },
                );
            });
        });
    }
}

fn compare_to_best<C, O, S>(heuristic_ctx: &C, solution: &S) -> Ordering
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx
        .ranked()
        .next()
        .map(|best_known| heuristic_ctx.objective().total_order(solution, best_known))
        .unwrap_or(Ordering::Less)
}

/// Estimates new solution discovery reward based on distance metric.
/// Returns `Some(reward)` for improvement, or `None` for stagnation.
fn estimate_distance_reward<C, O, S>(heuristic_ctx: &C, initial_solution: &S, new_solution: &S) -> Option<Float>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx
        .ranked()
        .next()
        .map(|best_known| {
            let objective = heuristic_ctx.objective();

            // Calculate normalized relative distances [0, 1].
            let distance_initial = get_relative_distance(objective, new_solution, initial_solution);
            let distance_best = get_relative_distance(objective, new_solution, best_known);

            // Reward components (max ~4.0 for local, ~10.0 for global).
            let reward_initial = (distance_initial + SearchRewards::DISTANCE_OFFSET) * SearchRewards::BASE_REWARD;
            let reward_best = (distance_best + SearchRewards::DISTANCE_OFFSET)
                * SearchRewards::BASE_REWARD
                * SearchRewards::GLOBAL_BEST_MULTIPLIER;

            match (distance_initial.total_cmp(&0.), distance_best.total_cmp(&0.)) {
                // Global Jackpot
                (Ordering::Greater, Ordering::Greater) => Some(reward_initial + reward_best),

                // Local/Diverse Improvement
                (Ordering::Greater, _) => Some(reward_initial * SearchRewards::DIVERSE_MULTIPLIER),

                // Stagnation
                _ => None,
            }
        })
        .unwrap_or(None)
}

/// Estimates performance of the used operation based on its duration and the current search phase.
/// Returns a reward multiplier in the range `[0.8, 1.2]`.
fn estimate_reward_perf_multiplier<C, O, S>(search_ctx: &SearchContext<C, O, S>, duration: usize) -> Float
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let stats = search_ctx.heuristic_ctx.statistics();
    let improvement_ratio = stats.improvement_1000_ratio;

    let approx_median = &search_ctx.approx_median;
    let median = match approx_median {
        Some(m) if *m > 0 => *m as Float,
        _ => return 1.0,
    };

    let time_ratio = duration as Float / median;

    // Calculate the raw time modifier (logarithmic).
    let raw_modifier = (1.0 / time_ratio).ln() * 0.15;

    // Apply phase-dependent damping.
    // Smooth transition from 0.0 to 1.0 based on improvement ratio.
    // We saturate at 0.1 (10% improvement is considered "Fast Flow").
    let phase_damping = (improvement_ratio * 10.0).clamp(0.0, 1.0);
    let final_modifier = raw_modifier * phase_damping;

    // Final safety clamp defined by reward physics.
    let tolerance = SearchRewards::PERF_TOLERANCE;
    (1.0 + final_modifier).clamp(1.0 - tolerance, 1.0 + tolerance)
}

/// Returns the normalized distance in `[0.0, 1.0]` (absolute magnitude)
/// where 1.0 = Improvement on Primary Objective.
/// Returns a negative value if worse.
fn get_relative_distance<O, S>(objective: &O, a: &S, b: &S) -> Float
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let order = objective.total_order(a, b);

    let sign = match order {
        Ordering::Less => 1.,
        Ordering::Greater => -1.,
        Ordering::Equal => return 0.,
    };

    let idx = a
        .fitness()
        .zip(b.fitness())
        .enumerate()
        .find(|(_, (fitness_a, fitness_b))| fitness_a != fitness_b)
        .map(|(idx, _)| idx);

    let idx = if let Some(idx) = idx {
        idx
    } else {
        return 0.;
    };

    let total_objectives = a.fitness().count();
    assert_ne!(total_objectives, 0, "cannot have an empty objective here");
    assert_ne!(total_objectives, idx, "cannot have the index equal to total amount of objectives");

    // Normalization: Divide by total_objectives to map to [0, 1].
    let priority_amplifier = (total_objectives - idx) as Float / total_objectives as Float;

    let value = a
        .fitness()
        .nth(idx)
        .zip(b.fitness().nth(idx))
        .map(|(a, b)| (a - b).abs() / a.abs().max(b.abs()))
        .expect("cannot get fitness by idx");

    value * sign * priority_amplifier
}

/// Signal tracker that observes ONLY positive values to establish a baseline for "Success".
/// Uses exponential moving average for stability in sparse signals.
#[derive(Clone)]
struct SignalStats {
    mean: Float,
    n: Float,
}

impl SignalStats {
    fn new() -> Self {
        Self { mean: 0.0, n: 0.0 }
    }

    /// Observes ONLY positive values to establish a baseline for "Success".
    fn update(&mut self, value: Float) {
        if value <= f64::EPSILON {
            return;
        }

        // Horizon: Adapt to the scale of the last ~200 successful operations.
        // This is structural (adaptation speed), not problem-specific.
        let window_size = 200.0;
        let decay = 1.0 - (1.0 / window_size);

        self.n = self.n * decay + 1.0;

        // Exponential moving average of the magnitude.
        // We use this instead of Welford's variance for stability in sparse signals.
        let learning_rate = 1.0 / self.n;
        self.mean = self.mean * (1.0 - learning_rate) + value * learning_rate;
    }

    /// Returns the scale factor.
    /// If we haven't seen enough data, return 1.0 to avoid division by zero.
    fn scale(&self) -> Float {
        if self.mean < f64::EPSILON { 1.0 } else { self.mean }
    }
}

/// Enhanced diagnostic tracker for Thompson sampling analysis.
struct HeuristicTracker {
    total_median: RemedianUsize,
    search_telemetry: Vec<(usize, SearchSample)>,
    heuristic_telemetry: Vec<(usize, HeuristicSample)>,
    is_experimental: bool,
}

impl HeuristicTracker {
    /// Creates a new tracker with diagnostic configuration.
    pub fn new(is_experimental: bool) -> Self {
        Self {
            total_median: RemedianUsize::new(11, 7, |a, b| a.cmp(b)),
            search_telemetry: Default::default(),
            heuristic_telemetry: Default::default(),
            is_experimental,
        }
    }

    /// Returns true if telemetry is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.is_experimental
    }

    /// Returns the median approximation.
    pub fn approx_median(&self) -> Option<usize> {
        self.total_median.approx_median()
    }

    /// Observes the current sample and updates the total duration median.
    pub fn observe_sample(&mut self, generation: usize, sample: SearchSample) {
        self.total_median.add_observation(sample.duration);
        if self.telemetry_enabled() {
            self.search_telemetry.push((generation, sample));
        }
    }

    /// Observes heuristic parameters for telemetry tracking.
    pub fn observe_params(&mut self, generation: usize, sample: HeuristicSample) {
        if self.telemetry_enabled() {
            self.heuristic_telemetry.push((generation, sample));
        }
    }
}

/// A sample of search telemetry.
#[derive(Clone)]
struct SearchSample {
    name: String,
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
