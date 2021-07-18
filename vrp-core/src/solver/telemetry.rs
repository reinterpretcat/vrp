//! A module which provides the logic to collect metrics about algorithm execution and simple logging.

#[cfg(test)]
#[path = "../../tests/unit/solver/telemetry_test.rs"]
mod telemetry_test;

use crate::algorithms::nsga2::Objective;
use crate::construction::heuristics::InsertionContext;
use crate::solver::population::SelectionPhase;
use crate::solver::{RefinementContext, RefinementSpeed, Statistics};
use crate::utils::{InfoLogger, Timer};
use std::fmt::Write;
use std::ops::Deref;

/// Encapsulates different measurements regarding algorithm evaluation.
pub struct Metrics {
    /// Algorithm duration.
    pub duration: usize,
    /// Total amount of generations.
    pub generations: usize,
    /// Speed: generations per second.
    pub speed: f64,
    /// Evolution progress.
    pub evolution: Vec<Generation>,
}

/// Represents information about generation.
pub struct Generation {
    /// Generation sequence number.
    pub number: usize,
    /// Time since evolution started.
    pub timestamp: f64,
    /// Overall improvement ratio.
    pub i_all_ratio: f64,
    /// Improvement ratio last 1000 generations.
    pub i_1000_ratio: f64,
    /// True if this generation considered as improvement.
    pub is_improvement: bool,
    /// Population state.
    pub population: Population,
}

/// Keeps essential information about particular individual in population.
pub struct Individual {
    /// Rank in population.
    pub rank: usize,
    /// Total amount of tours.
    pub tours: usize,
    /// Total amount of unassigned jobs.
    pub unassigned: usize,
    /// Solution cost.
    pub cost: f64,
    /// Solution improvement from best individual.
    pub improvement: f64,
    /// Objectives fitness values.
    pub fitness: Vec<f64>,
}

/// Holds population state.
pub struct Population {
    /// Population individuals.
    pub individuals: Vec<Individual>,
}

/// Specifies a telemetry mode.
pub enum TelemetryMode {
    /// No telemetry at all.
    None,
    /// Only logging.
    OnlyLogging {
        /// A logger type.
        logger: InfoLogger,
        /// Specifies how often best individual is logged.
        log_best: usize,
        /// Specifies how often population is logged.
        log_population: usize,
        /// Specifies whether population should be dumped.
        dump_population: bool,
    },
    /// Only metrics collection.
    OnlyMetrics {
        /// Specifies how often population is tracked.
        track_population: usize,
    },
    /// Both logging and metrics collection.
    All {
        /// A logger type.
        logger: InfoLogger,
        /// Specifies how often best individual is logged.
        log_best: usize,
        /// Specifies how often population is logged.
        log_population: usize,
        /// Specifies how often population is tracked.
        track_population: usize,
        /// Specifies whether population should be dumped.
        dump_population: bool,
    },
}

/// Provides way to collect metrics and write information into log.
pub struct Telemetry {
    metrics: Metrics,
    time: Timer,
    mode: TelemetryMode,
    improvement_tracker: ImprovementTracker,
    speed_tracker: SpeedTracker,
    next_generation: Option<usize>,
}

impl Telemetry {
    /// Creates a new instance of `Telemetry`.
    pub fn new(mode: TelemetryMode) -> Self {
        Self {
            time: Timer::start(),
            metrics: Metrics { duration: 0, generations: 0, speed: 0.0, evolution: vec![] },
            mode,
            improvement_tracker: ImprovementTracker::new(1000),
            speed_tracker: SpeedTracker::default(),
            next_generation: None,
        }
    }

    /// Starts telemetry reporting.
    pub fn start(&mut self) {
        self.time = Timer::start();
    }

    /// Reports initial solution statistics.
    pub fn on_initial(&mut self, item_idx: usize, total_items: usize, item_time: Timer, termination_estimate: f64) {
        match &self.mode {
            TelemetryMode::OnlyLogging { .. } | TelemetryMode::All { .. } => self.log(
                format!(
                    "[{}s] created {} of {} initial solutions in {}ms (ts: {})",
                    self.time.elapsed_secs(),
                    item_idx + 1,
                    total_items,
                    item_time.elapsed_millis(),
                    termination_estimate,
                )
                .as_str(),
            ),
            _ => {}
        };
    }

    /// Reports generation statistics.
    pub fn on_generation(
        &mut self,
        refinement_ctx: &mut RefinementContext,
        termination_estimate: f64,
        generation_time: Timer,
        is_improved: bool,
    ) {
        let generation = self.next_generation.unwrap_or(0);

        self.metrics.generations = generation;
        self.improvement_tracker.track(generation, is_improved);
        self.speed_tracker.track(generation, termination_estimate);

        refinement_ctx.statistics = Statistics {
            generation,
            time: self.time.clone(),
            speed: self.speed_tracker.get_current_speed(),
            improvement_all_ratio: self.improvement_tracker.i_all_ratio,
            improvement_1000_ratio: self.improvement_tracker.i_1000_ratio,
            termination_estimate,
        };

        self.next_generation = Some(generation + 1);

        let (log_best, log_population, track_population, should_dump_population) = match &self.mode {
            TelemetryMode::None => return,
            TelemetryMode::OnlyLogging { log_best, log_population, dump_population, .. } => {
                (Some(log_best), Some(log_population), None, *dump_population)
            }
            TelemetryMode::OnlyMetrics { track_population, .. } => (None, None, Some(track_population), false),
            TelemetryMode::All { log_best, log_population, track_population, dump_population, .. } => {
                (Some(log_best), Some(log_population), Some(track_population), *dump_population)
            }
        };

        if let Some((best_individual, rank)) = refinement_ctx.population.ranked().next() {
            let should_log_best = generation % *log_best.unwrap_or(&usize::MAX) == 0;
            let should_log_population = generation % *log_population.unwrap_or(&usize::MAX) == 0;
            let should_track_population = generation % *track_population.unwrap_or(&usize::MAX) == 0;

            if should_log_best {
                self.log_individual(
                    &self.get_individual_metrics(refinement_ctx, &best_individual, rank),
                    Some((refinement_ctx.statistics.generation, generation_time)),
                )
            }

            self.on_population(&refinement_ctx, should_log_population, should_track_population, should_dump_population);
        } else {
            self.log("no progress yet");
        }
    }

    /// Reports population state.
    fn on_population(
        &mut self,
        refinement_ctx: &RefinementContext,
        should_log_population: bool,
        should_track_population: bool,
        should_dump_population: bool,
    ) {
        if !should_log_population && !should_track_population {
            return;
        }

        if should_log_population {
            self.log(
                format!(
                    "[{}s] population state (phase: {}, speed: {:.2} gen/sec, improvement ratio: {:.3}:{:.3}):",
                    self.time.elapsed_secs(),
                    Self::get_selection_phase(refinement_ctx),
                    refinement_ctx.statistics.generation as f64 / self.time.elapsed_secs_as_f64(),
                    self.improvement_tracker.i_all_ratio,
                    self.improvement_tracker.i_1000_ratio,
                )
                .as_str(),
            );
        }

        let individuals = refinement_ctx
            .population
            .ranked()
            .map(|(insertion_ctx, rank)| self.get_individual_metrics(refinement_ctx, &insertion_ctx, rank))
            .collect::<Vec<_>>();

        if should_log_population {
            individuals.iter().for_each(|metrics| self.log_individual(&metrics, None));
            if should_dump_population {
                self.log(&format!("\t{}", Self::get_population_state(refinement_ctx)));
            }
        }

        if should_track_population {
            self.metrics.evolution.push(Generation {
                number: refinement_ctx.statistics.generation,
                timestamp: self.time.elapsed_secs_as_f64(),
                i_all_ratio: self.improvement_tracker.i_all_ratio,
                i_1000_ratio: self.improvement_tracker.i_1000_ratio,
                is_improvement: self.improvement_tracker.is_last_improved,
                population: Population { individuals },
            });
        }
    }

    /// Reports final statistic.
    pub fn on_result(&mut self, refinement_ctx: &RefinementContext) {
        let generations = refinement_ctx.statistics.generation;

        let (should_log_population, should_track_population) = match &self.mode {
            TelemetryMode::OnlyLogging { .. } => (true, false),
            TelemetryMode::OnlyMetrics { track_population, .. } => (false, generations % track_population != 0),
            TelemetryMode::All { track_population, .. } => (true, generations % track_population != 0),
            _ => return,
        };

        self.on_population(refinement_ctx, should_log_population, should_track_population, false);

        let elapsed = self.time.elapsed_secs() as usize;
        let speed = refinement_ctx.statistics.generation as f64 / self.time.elapsed_secs_as_f64();

        self.log(format!("[{}s] total generations: {}, speed: {:.2} gen/sec", elapsed, generations, speed).as_str());

        self.metrics.duration = elapsed;
        self.metrics.speed = speed;
    }

    /// Gets metrics.
    pub fn get_metrics(self) -> Option<Metrics> {
        match &self.mode {
            TelemetryMode::OnlyMetrics { .. } | TelemetryMode::All { .. } => Some(self.metrics),
            _ => None,
        }
    }

    /// Writes log message.
    pub fn log(&self, message: &str) {
        match &self.mode {
            TelemetryMode::OnlyLogging { logger, .. } => logger.deref()(message),
            TelemetryMode::All { logger, .. } => logger.deref()(message),
            _ => {}
        }
    }

    fn get_individual_metrics(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
        rank: usize,
    ) -> Individual {
        let fitness_values = insertion_ctx.get_fitness_values().collect::<Vec<_>>();

        let (cost, cost_difference) = Self::get_fitness(refinement_ctx, insertion_ctx);

        Individual {
            rank,
            tours: insertion_ctx.solution.routes.len(),
            unassigned: insertion_ctx.solution.unassigned.len(),
            cost,
            improvement: cost_difference,
            fitness: fitness_values,
        }
    }

    fn log_individual(&self, metrics: &Individual, gen_info: Option<(usize, Timer)>) {
        self.log(
            format!(
                "{} rank: {}, cost: {:.2}({:.3}%), tours: {}, unassigned: {}, fitness: ({})",
                gen_info.map_or("\t".to_string(), |(gen, gen_time)| format!(
                    "[{}s] generation {} took {}ms,",
                    self.time.elapsed_secs(),
                    gen,
                    gen_time.elapsed_millis()
                )),
                metrics.rank,
                metrics.cost,
                metrics.improvement,
                metrics.tours,
                metrics.unassigned,
                metrics.fitness.iter().map(|v| format!("{:.3}", v)).collect::<Vec<_>>().join(", ")
            )
            .as_str(),
        );
    }

    fn get_fitness(refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> (f64, f64) {
        let fitness_value = refinement_ctx.problem.objective.fitness(insertion_ctx);

        let fitness_change = refinement_ctx
            .population
            .ranked()
            .next()
            .map(|(best_ctx, _)| refinement_ctx.problem.objective.fitness(best_ctx))
            .map(|best_fitness| (fitness_value - best_fitness) / best_fitness * 100.)
            .unwrap_or(0.);

        (fitness_value, fitness_change)
    }

    fn get_population_state(refinement_ctx: &RefinementContext) -> String {
        let mut state = String::new();
        write!(state, "{}", refinement_ctx.population).unwrap();

        state
    }

    fn get_selection_phase(refinement_ctx: &RefinementContext) -> &str {
        match refinement_ctx.population.selection_phase() {
            SelectionPhase::Initial => "initial",
            SelectionPhase::Exploration => "exploration",
            SelectionPhase::Exploitation => "exploitation",
        }
    }
}

struct ImprovementTracker {
    buffer: Vec<bool>,
    total_improvements: usize,

    pub i_all_ratio: f64,
    pub i_1000_ratio: f64,
    pub is_last_improved: bool,
}

impl ImprovementTracker {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![false; size],
            total_improvements: 0,
            i_all_ratio: 0.,
            i_1000_ratio: 0.,
            is_last_improved: false,
        }
    }

    pub fn track(&mut self, generation: usize, is_improved: bool) {
        let length = self.buffer.len();

        if is_improved {
            self.total_improvements += 1;
        }

        self.is_last_improved = is_improved;
        self.buffer[generation % length] = is_improved;

        let improvements = (0..generation + 1).zip(self.buffer.iter()).filter(|(_, is_improved)| **is_improved).count();

        self.i_all_ratio = (self.total_improvements as f64) / ((generation + 1) as f64);
        self.i_1000_ratio = (improvements as f64) / ((generation + 1).min(self.buffer.len()) as f64);
    }
}

struct SpeedTracker {
    initial: f64,
    speed: RefinementSpeed,
}

impl Default for SpeedTracker {
    fn default() -> Self {
        Self { initial: 0., speed: RefinementSpeed::Moderate }
    }
}

impl SpeedTracker {
    pub fn track(&mut self, generation: usize, termination_estimate: f64) {
        if generation == 0 {
            self.initial = termination_estimate;
        } else {
            let delta = (termination_estimate - self.initial).max(0.);

            if generation < 200 && delta > 0.1 {
                self.speed = RefinementSpeed::Slow;
            }
        }
    }

    pub fn get_current_speed(&self) -> RefinementSpeed {
        self.speed.clone()
    }
}
