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

/// Base reward for finding a new global best solution.
/// This is the "jackpot" that operators compete for.
const JACKPOT_BASE: Float = 2.0;

/// Maximum reward for diverse improvements (soft ceiling via tanh saturation).
/// Must be well below JACKPOT_BASE to ensure exploitation of best-finding operators.
/// At 1.0, jackpots are guaranteed to be at least 2x more valuable than any diverse improvement.
const DIVERSE_CAP: Float = 1.0;

/// Minimum reward for any improvement (prevents near-zero rewards).
/// Set high enough to be a meaningful positive signal.
const MIN_REWARD: Float = 0.1;

/// Penalty scale for failures (multiplied by time ratio).
/// A failure that takes median time costs -0.1; twice median costs -0.2.
const PENALTY_SCALE: Float = 0.1;

/// Floor for rewards (maximum penalty).
const REWARD_MIN: Float = -1.0;

/// Ceiling for rewards (maximum jackpot + efficiency bonus).
const REWARD_MAX: Float = 10.0;

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

        // Compute reward using the simplified V2.1 formula.
        let reward =
            compute_reward(context.heuristic_ctx, context.solution, &new_solution, duration, context.approx_median);

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
    /// Separate learning contexts for different search phases (BestKnown vs Diverse).
    slot_machines: HashMap<SearchState, SlotMachines<'a, C, O, S>>,
    /// Tracks operator durations for median calculation.
    tracker: HeuristicTracker,
    /// Random number generator for Thompson sampling selection.
    random: Arc<dyn Random>,
}

impl<'a, C, O, S> SearchAgent<'a, C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'a,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution + 'a,
{
    pub fn new(search_operators: HeuristicSearchOperators<C, O, S>, environment: &Environment) -> Self {
        // Normalize weights so the average operator has prior_mean ≈ 1.0.
        // This aligns with typical success rewards (~1-3 range).
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
                    // Smooth mapping of weight ratio to prior mean range [0.1, 3.0].
                    // Uses tanh for smooth compression without hard cutoffs.
                    // ratio=1 (average) → prior=1.0, higher ratios → up to 3.0, lower → down to 0.1
                    let ratio = initial_weight / avg_weight;
                    let t = (ratio - 1.0).tanh(); // smooth compression to [-1, 1]
                    // Asymmetric scaling: [−1,0] → [0.1,1.0], [0,1] → [1.0,3.0]
                    let prior_mean = if t >= 0.0 { 1.0 + t * 2.0 } else { 1.0 + t * 0.9 };
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

        // Initialize separate states with identical priors but independent learning.
        let slot_machines = once((SearchState::BestKnown, create_slots()))
            .chain(once((SearchState::Diverse, create_slots())))
            .collect();

        Self {
            slot_machines,
            tracker: HeuristicTracker::new(environment.is_experimental),
            random: environment.random.clone(),
        }
    }

    /// Picks the relevant search operator using pure Thompson Sampling and runs the search.
    pub fn search(&self, heuristic_ctx: &C, solution: &S) -> SearchFeedback<S> {
        // Determine search context - critical for operator selection.
        let from = if matches!(compare_to_best(heuristic_ctx, solution), Ordering::Equal) {
            SearchState::BestKnown
        } else {
            SearchState::Diverse
        };

        // Get contextually appropriate slot machines.
        let slots = self.slot_machines.get(&from).expect("cannot get slot machines");

        // Sample each arm, pick argmax with random tie-break.
        let samples = slots.iter().map(|(slot, _)| slot.sample()).collect::<Vec<_>>();
        let slot_idx = random_argmax(samples.into_iter(), self.random.as_ref()).unwrap_or(0);
        let slot_machine = &slots[slot_idx].0;

        let approx_median = self.tracker.approx_median();

        // Execute with full context information.
        slot_machine.play(SearchContext { heuristic_ctx, from, slot_idx, solution, approx_median })
    }

    /// Updates the slot machine with the raw reward (no normalization needed).
    pub fn update(&mut self, generation: usize, feedback: &SearchFeedback<S>) {
        let from = &feedback.sample.transition.0;
        let slots = self.slot_machines.get_mut(from).expect("cannot get slot machines");
        slots[feedback.slot_idx].0.update(feedback);

        // Track telemetry.
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

/// Computes the reward for an operator based on solution improvement.
///
/// Key design principles:
/// 1. **Best-Known Anchoring**: Improvements far from best get diminished credit.
/// 2. **Stagnation-Aware Efficiency**: Slow operators tolerated during stagnation.
/// 3. **Bounded Output**: Rewards in [-1, 5] for stable Bayesian updates.
fn compute_reward<C, O, S>(
    heuristic_ctx: &C,
    initial_solution: &S,
    new_solution: &S,
    duration: usize,
    approx_median: Option<usize>,
) -> Float
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let objective = heuristic_ctx.objective();

    // Get best known solution for anchoring.
    let best_known = match heuristic_ctx.ranked().next() {
        Some(best) => best,
        None => return 0.0, // No population yet, neutral reward.
    };

    // Determine improvement types.
    let is_new_best = objective.total_order(new_solution, best_known) == Ordering::Less;
    let is_improvement = objective.total_order(new_solution, initial_solution) == Ordering::Less;

    // COMPUTE REWARD BASED ON IMPROVEMENT TYPE
    let raw_reward = if is_new_best {
        // JACKPOT: Found new global best!
        // Apply magnitude scaling so small improvements become meaningful signals.
        // Raw distance is often tiny (0.001 for 0.1% improvement).
        // ln_1p(x * 1000) transforms: 0.001 -> ~0.69, 0.01 -> ~2.4, 0.1 -> ~4.6
        let improvement_distance = get_relative_distance(new_solution, initial_solution);
        let magnitude = (improvement_distance * 1000.0).ln_1p();
        JACKPOT_BASE + magnitude
    } else if is_improvement {
        // DIVERSE IMPROVEMENT: Better than starting point, but not global best.
        // Use tanh for soft saturation - large improvements asymptote to DIVERSE_CAP,
        // ensuring diverse rewards stay well below JACKPOT_BASE.
        let improvement_distance = get_relative_distance(new_solution, initial_solution);
        let magnitude = (improvement_distance * 1000.0).ln_1p();
        let saturated = magnitude.tanh();

        // Proximity to best: closer solutions get higher reward.
        let gap_to_best = get_relative_distance(new_solution, best_known);
        let proximity_factor = (1.0 - gap_to_best).powi(2);

        // Base utility: soft-capped and bounded [MIN_REWARD, DIVERSE_CAP].
        let base_utility = (DIVERSE_CAP * saturated * proximity_factor).clamp(MIN_REWARD, DIVERSE_CAP);

        // Apply efficiency modulation: fast improvements get bonus, slow ones get penalty.
        let median = approx_median.unwrap_or(duration.max(1)).max(1) as Float;
        let improvement_ratio = heuristic_ctx.statistics().improvement_1000_ratio;

        // "Flow" measures search progress: 0 = stagnating, 1 = fast progress.
        let flow = (improvement_ratio * 10.0).clamp(0.0, 1.0);

        // Efficiency clamp range adapts to search phase (reduced impact):
        // - Stagnation (flow=0): [0.9, 1.1] - 10% penalty to 10% bonus
        // - Fast progress (flow=1): [0.8, 1.2] - 20% penalty to 20% bonus
        let min_eff = 0.9 - flow * 0.1;
        let max_eff = 1.1 + flow * 0.1;

        let efficiency = (median / duration as Float).clamp(min_eff, max_eff);

        base_utility * efficiency
    } else {
        // FAILURE: time-proportional penalty.
        let median = approx_median.unwrap_or(duration.max(1)).max(1) as Float;
        let time_ratio = duration as Float / median;
        -PENALTY_SCALE * time_ratio
    };

    // Clamp to bounded range for stable Bayesian updates.
    raw_reward.clamp(REWARD_MIN, REWARD_MAX)
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

/// Returns the normalized distance in `[0.0, 1.0]`.
fn get_relative_distance<S>(a: &S, b: &S) -> Float
where
    S: HeuristicSolution,
{
    // Find the first differing fitness component.
    let idx = a
        .fitness()
        .zip(b.fitness())
        .enumerate()
        .find(|(_, (fitness_a, fitness_b))| fitness_a != fitness_b)
        .map(|(idx, _)| idx);

    let idx = match idx {
        Some(idx) => idx,
        None => return 0., // All fitness values equal.
    };

    let total_objectives = a.fitness().count();
    if total_objectives == 0 || total_objectives == idx {
        return 0.;
    }

    // Priority amplifier: earlier objectives matter more.
    let priority_amplifier = (total_objectives - idx) as Float / total_objectives as Float;

    // Relative difference in the differing component.
    let value = a
        .fitness()
        .nth(idx)
        .zip(b.fitness().nth(idx))
        .map(|(a, b)| (a - b).abs() / a.abs().max(b.abs()).max(f64::EPSILON))
        .unwrap_or(0.0);

    value * priority_amplifier
}

/// Diagnostic tracker for Thompson sampling analysis.
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

        // To avoid WASM memory issues, downsample telemetry while preserving analysis capability
        // Strategy: keep early samples, sample periodically, and keep recent samples
        const MAX_SAMPLES: usize = 5000;
        const EARLY_SAMPLES: usize = 500;
        const RECENT_SAMPLES: usize = 500;

        f.write_fmt(format_args!("TELEMETRY\n"))?;
        f.write_fmt(format_args!("search:\n"))?;
        f.write_fmt(format_args!("name,generation,reward,from,to,duration\n"))?;

        let search_total = self.agent.tracker.search_telemetry.len();
        if search_total <= MAX_SAMPLES {
            // Small enough, output all
            for (generation, sample) in self.agent.tracker.search_telemetry.iter() {
                f.write_fmt(format_args!(
                    "{},{},{},{},{},{}\n",
                    sample.name, generation, sample.reward, sample.transition.0, sample.transition.1, sample.duration
                ))?;
            }
        } else {
            // Downsample: early + periodic middle + recent
            let middle_samples = MAX_SAMPLES - EARLY_SAMPLES - RECENT_SAMPLES;
            let middle_start = EARLY_SAMPLES;
            let middle_end = search_total - RECENT_SAMPLES;
            let step = (middle_end - middle_start) / middle_samples;

            for (i, (generation, sample)) in self.agent.tracker.search_telemetry.iter().enumerate() {
                let include = i < EARLY_SAMPLES
                    || i >= search_total - RECENT_SAMPLES
                    || (i >= middle_start && i < middle_end && (i - middle_start).is_multiple_of(step));

                if include {
                    f.write_fmt(format_args!(
                        "{},{},{},{},{},{}\n",
                        sample.name,
                        generation,
                        sample.reward,
                        sample.transition.0,
                        sample.transition.1,
                        sample.duration
                    ))?;
                }
            }
        }

        f.write_fmt(format_args!("heuristic:\n"))?;
        f.write_fmt(format_args!("generation,state,name,alpha,beta,mu,v,n\n"))?;

        let heuristic_total = self.agent.tracker.heuristic_telemetry.len();
        if heuristic_total <= MAX_SAMPLES {
            // Small enough, output all
            for (generation, sample) in self.agent.tracker.heuristic_telemetry.iter() {
                f.write_fmt(format_args!(
                    "{},{},{},{},{},{},{},{}\n",
                    generation, sample.state, sample.name, sample.alpha, sample.beta, sample.mu, sample.v, sample.n
                ))?;
            }
        } else {
            // Downsample: early + periodic middle + recent
            let middle_samples = MAX_SAMPLES - EARLY_SAMPLES - RECENT_SAMPLES;
            let middle_start = EARLY_SAMPLES;
            let middle_end = heuristic_total - RECENT_SAMPLES;
            let step = (middle_end - middle_start) / middle_samples;

            for (i, (generation, sample)) in self.agent.tracker.heuristic_telemetry.iter().enumerate() {
                let include = i < EARLY_SAMPLES
                    || i >= heuristic_total - RECENT_SAMPLES
                    || (i >= middle_start && i < middle_end && (i - middle_start).is_multiple_of(step));

                if include {
                    f.write_fmt(format_args!(
                        "{},{},{},{},{},{},{},{}\n",
                        generation, sample.state, sample.name, sample.alpha, sample.beta, sample.mu, sample.v, sample.n
                    ))?;
                }
            }
        }

        Ok(())
    }
}
