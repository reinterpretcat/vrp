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
/// learning technics.
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

/// Type alias for slot machines used in Thompson sampling
pub type SlotMachines<'a, C, O, S> = Vec<(SlotMachine<SearchAction<'a, C, O, S>, DefaultDistributionSampler>, String)>;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Search state for Thompson sampling
pub enum SearchState {
    /// Best known solution state
    BestKnown,
    /// Diverse solution state
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

/// Search feedback result for Thompson sampling
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

/// Search action wrapper for Thompson sampling
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

        let is_new_best = compare_to_best(context.heuristic_ctx, &new_solution) == Ordering::Less;
        let duration = duration.as_millis() as usize;

        let base_reward = estimate_distance_reward(context.heuristic_ctx, context.solution, &new_solution);
        let reward_multiplier = estimate_reward_perf_multiplier(&context, duration);
        let reward = base_reward * reward_multiplier;

        let to = if is_new_best { SearchState::BestKnown } else { SearchState::Diverse };
        let transition = (context.from, to);

        let sample = SearchSample { name: self.operator_name.clone(), duration, reward, transition };

        SearchFeedback { sample, slot_idx: context.slot_idx, solution: Some(new_solution) }
    }
}

/// Search context for Thompson sampling
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
        // Calculate scaling factor to normalize operator weights to target range
        let max_weight = search_operators.iter().map(|(_, _, weight)| *weight).fold(0.0_f64, |a, b| a.max(b));
        const TARGET_MAX_PRIOR: Float = 4.0;
        let scale = if max_weight > 0.0 { TARGET_MAX_PRIOR / max_weight } else { 1.0 };

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

    /// Picks relevant search operator based on learnings and runs the search.
    pub fn search(&self, heuristic_ctx: &C, solution: &S) -> SearchFeedback<S> {
        // Determine search context - critical for operator selection
        let from = if matches!(compare_to_best(heuristic_ctx, solution), Ordering::Equal) {
            SearchState::BestKnown
        } else {
            SearchState::Diverse
        };

        // Get contextually appropriate slot machines
        let (slot_idx, slot_machine) = self
            .slot_machines
            .get(&from)
            .and_then(|slots| {
                random_argmax(slots.iter().map(|(slot, _)| slot.sample()), self.random.as_ref())
                    .and_then(|slot_idx| slots.get(slot_idx).map(|(slot, _)| (slot_idx, slot)))
            })
            .expect("cannot get slot machine");

        let approx_median = self.tracker.approx_median();

        // Execute with full context information
        slot_machine.play(SearchContext { heuristic_ctx, from, slot_idx, solution, approx_median })
    }

    /// Updates estimations based on search feedback with protected signal baseline.
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        let raw_reward = feedback.sample.reward;
        let current_scale = self.signal_stats.scale();

        // 1. Update Shared Signal Baseline (Protected)
        // Clamp observation to 3x current scale to prevent a single
        // "Best Known Jackpot" from disrupting baseline for Diverse state
        if raw_reward > f64::EPSILON {
            self.signal_stats.update(raw_reward.min(current_scale * 3.0));
        }

        // 2. Normalize for Contextual Learning (Uncapped)
        // If we hit a jackpot (e.g., reward 60.0 vs scale 1.0), we pass the full 60.0.
        // This ensures the specific slot machine learns "I AM THE BEST"
        let normalized_reward = if raw_reward > f64::EPSILON {
            raw_reward / self.signal_stats.scale()
        } else {
            // Zero reward strategy: don't penalize, just indicate no progress
            // Allows variance in "good" operators to keep them alive vs "bad" operators
            0.0
        };
        let normalized_feedback = SearchFeedback {
            sample: SearchSample { reward: normalized_reward, ..feedback.sample.clone() },
            slot_idx: feedback.slot_idx,
            solution: None,
        };

        // 3. Update Contextual Slot Machine
        let from = &feedback.sample.transition.0;
        let slots = self.slot_machines.get_mut(from).expect("cannot get slot machines");
        slots[feedback.slot_idx].0.update(&normalized_feedback);

        // 4. Observe telemetry
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

/// Sample of search telemetry.
#[derive(Clone)]
struct SearchSample {
    name: String,
    duration: usize,
    reward: Float,
    transition: (SearchState, SearchState),
}

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
/// Returns a reward estimation in `[0, 6]` range. This range consists of:
/// - a initial distance improvement gives `[0, 2]`
/// - a best known improvement gives `[0, 2]` * BEST_DISCOVERY_REWARD_MULTIPLIER
fn estimate_distance_reward<C, O, S>(heuristic_ctx: &C, initial_solution: &S, new_solution: &S) -> Float
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic_ctx
        .ranked()
        .next()
        .map(|best_known| {
            const BEST_DISCOVERY_REWARD_MULTIPLIER: Float = 2.;
            const DIVERSE_DISCOVERY_REWARD_MULTIPLIER: Float = 0.05;

            let objective = heuristic_ctx.objective();

            let distance_initial = get_relative_distance(objective, new_solution, initial_solution);
            let distance_best = get_relative_distance(objective, new_solution, best_known);

            // NOTE remap distances to range [0, 2]
            match (distance_initial.total_cmp(&0.), distance_best.total_cmp(&0.)) {
                (Ordering::Greater, Ordering::Greater) => {
                    (distance_initial + 1.) + (distance_best + 1.) * BEST_DISCOVERY_REWARD_MULTIPLIER
                }
                (Ordering::Greater, _) => (distance_initial + 1.) * DIVERSE_DISCOVERY_REWARD_MULTIPLIER,
                _ => 0.,
            }
        })
        .unwrap_or(0.)
}

/// Estimates performance of the used operation based on its duration and the current search phase.
/// Returns a reward multiplier in the range `[0.8, 1.2]`.
///
/// # Strategy: Context-Aware Time Dilation
///
/// This function balances **Throughput (Speed)** vs. **Depth (Quality)** by adapting to the
/// current `improvement_1000_ratio`:
///
/// 1.  **Flow State (High Improvement):** When the solver is finding frequent improvements,
///     we prioritize **Efficiency**. Fast operators are rewarded, and slow operators are penalized.
///     This maximizes the generation of diverse individuals to populate the pool.
/// 2.  **Stagnation (Low Improvement):** When the solver is stuck, we prioritize **Power**.
///     The time penalty is dampened or removed. This ensures that "Heavy" operators (e.g.,
///     complex Ruin & Recreate), which are naturally slower but capable of escaping local optima,
///     are not unfairly penalized against faster, ineffective local search operators.
///
/// # Logic
///
/// *   **Continuous Scaling:** Uses a logarithmic curve to avoid artificial "cliffs" in reward.
/// *   **Phase Damping:** The time modifier is scaled by the improvement ratio.
/// *   **Safety Clamp:** The final multiplier is bounded to `+/- 20%` to ensure that execution
///     time never overrides the actual quality signal (distance improvement).
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

    // 1. Calculate the raw time modifier (Logarithmic)
    // Fast (0.5x) -> +0.15 reward
    // Slow (2.0x) -> -0.15 reward
    // We use a gentle curve so we don't distort the signal too much.
    let raw_modifier = (1.0 / time_ratio).ln() * 0.15;

    // 2. Apply Phase-Dependent Damping
    // If improvement is HIGH (> 0.1), we care about speed. Damping = 1.0.
    // If improvement is LOW (< 0.001), we ignore speed. Damping = 0.0.
    // This allows slow, heavy operators to "catch up" in ranking when the easy gains are gone.

    // Smooth transition from 0.0 to 1.0 based on improvement ratio
    // We saturate at 0.1 (10% improvement is considered "Fast Flow")
    let phase_damping = (improvement_ratio * 10.0).clamp(0.0, 1.0);

    // Apply damping
    let final_modifier = raw_modifier * phase_damping;

    // 3. Final Safety Clamp
    // Ensure we never boost/penalize by more than 20%, regardless of how extreme the time is.
    (1.0 + final_modifier).clamp(0.8, 1.2)
}

/// Returns distance in `[-N., N]` where:
///  - N: in range `[1, total amount of objectives]`
///  - sign specifies whether a solution is better (positive) or worse (negative).
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
    let priority_amplifier = (total_objectives - idx) as Float;

    let value = a
        .fitness()
        .nth(idx)
        .zip(b.fitness().nth(idx))
        .map(|(a, b)| (a - b).abs() / a.abs().max(b.abs()))
        .expect("cannot get fitness by idx");

    value * sign * priority_amplifier
}

/// Signal tracker that observes ONLY positive values to establish baseline for "Success".
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

    /// Observe ONLY positive values to establish a baseline for "Success"
    fn update(&mut self, value: Float) {
        if value <= f64::EPSILON {
            return;
        }

        // Horizon: Adapt to the scale of the last ~200 SUCCESSFUL operations
        // This is structural (adaptation speed), not problem-specific.
        let window_size = 200.0;
        let decay = 1.0 - (1.0 / window_size);

        self.n = self.n * decay + 1.0;

        // Exponential Moving Average of the Magnitude
        // We use this instead of Welford's variance for stability in sparse signals
        let learning_rate = 1.0 / self.n;
        self.mean = self.mean * (1.0 - learning_rate) + value * learning_rate;
    }

    /// Returns the scale factor.
    /// If we haven't seen enough data, return 1.0 to avoid division by zero.
    fn scale(&self) -> Float {
        if self.mean < f64::EPSILON { 1.0 } else { self.mean }
    }
}

/// Enhanced diagnostic tracker for Thompson sampling analysis
struct HeuristicTracker {
    total_median: RemedianUsize,
    search_telemetry: Vec<(usize, SearchSample)>,
    heuristic_telemetry: Vec<(usize, HeuristicSample)>,
    is_experimental: bool,
}

impl HeuristicTracker {
    /// Creates new tracker with diagnostic configuration
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
