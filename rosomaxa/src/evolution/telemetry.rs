//! A module which provides the logic to collect metrics about algorithm execution and simple logging.

#[cfg(test)]
#[path = "../../tests/unit/evolution/telemetry_test.rs"]
mod telemetry_test;

use crate::prelude::*;
use crate::utils::Timer;
use crate::DynHeuristicPopulation;
use std::cmp::Ordering;
use std::fmt::Write;
use std::marker::PhantomData;
use std::ops::Deref;

/// Encapsulates different measurements regarding algorithm evaluation.
pub struct TelemetryMetrics {
    /// Algorithm duration.
    pub duration: usize,
    /// Total amount of generations.
    pub generations: usize,
    /// Speed: generations per second.
    pub speed: f64,
    /// Evolution progress.
    pub evolution: Vec<TelemetryGeneration>,
}

/// Represents information about generation.
pub struct TelemetryGeneration {
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
    pub population: TelemetryPopulation,
}

/// Keeps essential information about particular individual in population.
pub struct TelemetryIndividual {
    /// Rank in population.
    pub rank: usize,
    /// Solution difference from best individual.
    pub difference: f64,
    /// Objectives fitness values.
    pub fitness: Vec<f64>,
}

/// Holds population state.
pub struct TelemetryPopulation {
    /// Population individuals.
    pub individuals: Vec<TelemetryIndividual>,
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
pub struct Telemetry<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    metrics: TelemetryMetrics,
    time: Timer,
    mode: TelemetryMode,
    statistics: HeuristicStatistics,
    improvement_tracker: ImprovementTracker,
    speed_tracker: SpeedTracker,
    next_generation: Option<usize>,
    _marker: (PhantomData<O>, PhantomData<S>),
}

impl<O, S> Telemetry<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `Telemetry`.
    pub fn new(mode: TelemetryMode) -> Self {
        Self {
            time: Timer::start(),
            metrics: TelemetryMetrics { duration: 0, generations: 0, speed: 0.0, evolution: vec![] },
            mode,
            statistics: Default::default(),
            improvement_tracker: ImprovementTracker::new(1000),
            speed_tracker: SpeedTracker::default(),
            next_generation: None,
            _marker: Default::default(),
        }
    }

    /// Reports initial solution statistics.
    pub fn on_initial(&mut self, solution: &S, item_time: Timer) {
        match &self.mode {
            TelemetryMode::OnlyLogging { .. } | TelemetryMode::All { .. } => {
                self.log(
                    format!(
                        "[{}s] created initial solution in {}ms, fitness: ({})",
                        self.time.elapsed_secs(),
                        item_time.elapsed_millis(),
                        format_fitness(solution.get_fitness())
                    )
                    .as_str(),
                );
            }
            _ => {}
        };
    }

    /// Reports generation statistics.
    pub fn on_generation(
        &mut self,
        objective: &O,
        population: &DynHeuristicPopulation<O, S>,
        termination_estimate: f64,
        generation_time: Timer,
        is_improved: bool,
    ) {
        let generation = self.next_generation.unwrap_or(0);

        self.metrics.generations = generation;
        self.improvement_tracker.track(generation, is_improved);
        self.speed_tracker.track(generation, &self.time, termination_estimate);
        self.next_generation = Some(generation + 1);

        self.statistics = HeuristicStatistics {
            generation,
            time: self.time.clone(),
            speed: self.speed_tracker.get_current_speed(),
            improvement_all_ratio: self.improvement_tracker.i_all_ratio,
            improvement_1000_ratio: self.improvement_tracker.i_1000_ratio,
            termination_estimate,
        };

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

        if let Some((best_individual, rank)) = population.ranked().next() {
            let should_log_best = generation % *log_best.unwrap_or(&usize::MAX) == 0;
            let should_log_population = generation % *log_population.unwrap_or(&usize::MAX) == 0;
            let should_track_population = generation % *track_population.unwrap_or(&usize::MAX) == 0;

            if should_log_best {
                self.log_individual(
                    &self.get_individual_metrics(objective, population, best_individual, rank),
                    Some((generation, generation_time)),
                )
            }

            self.on_population(
                objective,
                population,
                should_log_population,
                should_track_population,
                should_dump_population,
            );
        } else {
            self.log("no progress yet");
        }
    }

    /// Reports population state.
    fn on_population(
        &mut self,
        objective: &O,
        population: &DynHeuristicPopulation<O, S>,
        should_log_population: bool,
        should_track_population: bool,
        should_dump_population: bool,
    ) {
        if !should_log_population && !should_track_population {
            return;
        }

        let generation = self.statistics.generation;

        if should_log_population {
            let selection_phase = match population.selection_phase() {
                SelectionPhase::Initial => "initial",
                SelectionPhase::Exploration => "exploration",
                SelectionPhase::Exploitation => "exploitation",
            };

            self.log(
                format!(
                    "[{}s] population state (phase: {}, speed: {:.2} gen/sec, improvement ratio: {:.3}:{:.3}):",
                    self.time.elapsed_secs(),
                    selection_phase,
                    generation as f64 / self.time.elapsed_secs_as_f64(),
                    self.improvement_tracker.i_all_ratio,
                    self.improvement_tracker.i_1000_ratio,
                )
                .as_str(),
            );
        }

        let individuals = population
            .ranked()
            .map(|(insertion_ctx, rank)| self.get_individual_metrics(objective, population, insertion_ctx, rank))
            .collect::<Vec<_>>();

        if should_log_population {
            individuals.iter().for_each(|metrics| self.log_individual(metrics, None));
            if should_dump_population {
                let mut state = String::new();
                write!(state, "{}", population).unwrap();
                self.log(&format!("\t{}", state));
            }
        }

        if should_track_population {
            self.metrics.evolution.push(TelemetryGeneration {
                number: generation,
                timestamp: self.time.elapsed_secs_as_f64(),
                i_all_ratio: self.improvement_tracker.i_all_ratio,
                i_1000_ratio: self.improvement_tracker.i_1000_ratio,
                is_improvement: self.improvement_tracker.is_last_improved,
                population: TelemetryPopulation { individuals },
            });
        }
    }

    /// Reports final statistic.
    pub fn on_result(&mut self, objective: &O, population: &DynHeuristicPopulation<O, S>) {
        let generations = self.statistics.generation;

        let (should_log_population, should_track_population) = match &self.mode {
            TelemetryMode::OnlyLogging { .. } => (true, false),
            TelemetryMode::OnlyMetrics { track_population, .. } => (false, generations % track_population != 0),
            TelemetryMode::All { track_population, .. } => (true, generations % track_population != 0),
            _ => return,
        };

        self.on_population(objective, population, should_log_population, should_track_population, false);

        let elapsed = self.time.elapsed_secs() as usize;
        let speed = generations as f64 / self.time.elapsed_secs_as_f64();

        self.log(format!("[{}s] total generations: {}, speed: {:.2} gen/sec", elapsed, generations, speed).as_str());

        self.metrics.duration = elapsed;
        self.metrics.speed = speed;
    }

    /// Gets metrics.
    pub fn take_metrics(self) -> Option<TelemetryMetrics> {
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

    /// Returns current statistics.
    pub fn get_statistics(&self) -> &HeuristicStatistics {
        &self.statistics
    }

    fn get_individual_metrics(
        &self,
        objective: &O,
        population: &DynHeuristicPopulation<O, S>,
        solution: &S,
        rank: usize,
    ) -> TelemetryIndividual {
        let fitness = solution.get_fitness().collect::<Vec<_>>();

        let (_, difference) = get_fitness_value(objective, population, solution);

        TelemetryIndividual { rank, difference: difference.abs(), fitness }
    }

    fn log_individual(&self, metrics: &TelemetryIndividual, gen_info: Option<(usize, Timer)>) {
        let fitness = format_fitness(metrics.fitness.iter().cloned());

        let value = if let Some((gen, gen_time)) = gen_info {
            format!(
                "[{}s] generation {} took {}ms, fitness: ({})",
                self.time.elapsed_secs(),
                gen,
                gen_time.elapsed_millis(),
                fitness
            )
        } else {
            format!("\trank: {}, fitness: ({}), difference: {:.3}%", metrics.rank, fitness, metrics.difference)
        };

        self.log(value.as_str());
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
    initial_estimate: f64,
    initial_time: f64,
    speed: HeuristicSpeed,
}

impl Default for SpeedTracker {
    fn default() -> Self {
        Self { initial_estimate: 0., initial_time: 0., speed: HeuristicSpeed::Unknown }
    }
}

impl SpeedTracker {
    pub fn track(&mut self, generation: usize, time: &Timer, termination_estimate: f64) {
        let elapsed = (time.elapsed_millis() as f64) * 1000.;
        if generation == 0 {
            self.initial_estimate = termination_estimate;
            self.initial_time = elapsed;
        } else {
            let average = (elapsed - self.initial_time) / generation as f64;

            let delta = (termination_estimate - self.initial_estimate).max(0.);
            let ratio = match (generation, delta) {
                (generation, delta) if generation < 10 && delta > 0.1 => 0.1,
                (generation, delta) if generation < 100 && delta > 0.1 => 0.25,
                (generation, delta) if generation < 200 && delta > 0.1 => 0.5,
                (generation, delta) if generation < 500 && delta > 0.2 => 0.5,
                _ => 1.,
            };
            let is_slow = compare_floats(ratio, 1.) == Ordering::Less && average < 5.;

            self.speed = match &self.speed {
                HeuristicSpeed::Unknown | HeuristicSpeed::Moderate { .. } if !is_slow => {
                    HeuristicSpeed::Moderate { average }
                }
                _ => HeuristicSpeed::Slow { ratio, average },
            }
        }
    }

    pub fn get_current_speed(&self) -> HeuristicSpeed {
        self.speed.clone()
    }
}

fn get_fitness_value<O, S>(objective: &O, population: &DynHeuristicPopulation<O, S>, solution: &S) -> (f64, f64)
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let fitness_value = objective.fitness(solution);

    let fitness_change = population
        .ranked()
        .next()
        .map(|(best_ctx, _)| objective.fitness(best_ctx))
        .map(|best_fitness| (fitness_value - best_fitness) / best_fitness * 100.)
        .unwrap_or(0.);

    (fitness_value, fitness_change)
}

fn format_fitness(fitness: impl Iterator<Item = f64>) -> String {
    fitness.map(|v| format!("{:.3}", v)).collect::<Vec<_>>().join(", ")
}
