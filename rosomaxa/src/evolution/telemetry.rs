//! A module which provides the logic to collect metrics about algorithm execution and simple logging.

#[cfg(test)]
#[path = "../../tests/unit/evolution/telemetry_test.rs"]
mod telemetry_test;

use crate::algorithms::math::relative_distance;
use crate::prelude::*;
use crate::utils::Timer;
use crate::{DynHeuristicPopulation, RemedianUsize};
use std::marker::PhantomData;

/// Encapsulates different measurements regarding algorithm evaluation.
pub struct TelemetryMetrics {
    /// Algorithm duration.
    pub duration: usize,
    /// Total amount of generations.
    pub generations: usize,
    /// Speed: generations per second.
    pub speed: Float,
    /// Evolution progress.
    pub evolution: Vec<TelemetryGeneration>,
}

/// Represents information about generation.
pub struct TelemetryGeneration {
    /// Generation sequence number.
    pub number: usize,
    /// Time since evolution started.
    pub timestamp: Float,
    /// Overall improvement ratio.
    pub i_all_ratio: Float,
    /// Improvement ratio last 1000 generations.
    pub i_1000_ratio: Float,
    /// True if this generation considered as improvement.
    pub is_improvement: bool,
    /// Population state.
    pub population: TelemetryPopulation,
}

/// Keeps essential information about particular individual in population.
pub struct TelemetryIndividual {
    /// Solution difference from best individual.
    pub difference: Float,
    /// Objectives fitness values.
    pub fitness: Vec<Float>,
}

/// Holds population state.
pub struct TelemetryPopulation {
    /// Population individuals.
    pub individuals: Vec<TelemetryIndividual>,
}

/// Specifies a telemetry mode.
#[derive(Clone)]
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
                        format_fitness(solution.fitness())
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
        population: &DynHeuristicPopulation<O, S>,
        termination_estimate: Float,
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

        let (log_best, log_population, track_population) = match &self.mode {
            TelemetryMode::None => return,
            TelemetryMode::OnlyLogging { log_best, log_population, .. } => (Some(log_best), Some(log_population), None),
            TelemetryMode::OnlyMetrics { track_population, .. } => (None, None, Some(track_population)),
            TelemetryMode::All { log_best, log_population, track_population, .. } => {
                (Some(log_best), Some(log_population), Some(track_population))
            }
        };

        match population.ranked().next() {
            Some(best_individual) => {
                let should_log_best = generation % *log_best.unwrap_or(&usize::MAX) == 0;
                let should_log_population = generation % *log_population.unwrap_or(&usize::MAX) == 0;
                let should_track_population = generation % *track_population.unwrap_or(&usize::MAX) == 0;

                if should_log_best {
                    self.log_individual(
                        &self.get_individual_metrics(population, best_individual),
                        Some((generation, generation_time)),
                    )
                }

                self.on_population(population, should_log_population, should_track_population);
            }
            _ => {
                self.log("no progress yet");
            }
        }
    }

    /// Reports population state.
    fn on_population(
        &mut self,
        population: &DynHeuristicPopulation<O, S>,
        should_log_population: bool,
        should_track_population: bool,
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
                    generation as Float / self.time.elapsed_secs_as_float(),
                    self.improvement_tracker.i_all_ratio,
                    self.improvement_tracker.i_1000_ratio,
                )
                .as_str(),
            );
        }

        let individuals = population.ranked().map(|s| self.get_individual_metrics(population, s)).collect::<Vec<_>>();

        if should_log_population {
            individuals.iter().for_each(|metrics| self.log_individual(metrics, None));
        }

        if should_track_population {
            self.metrics.evolution.push(TelemetryGeneration {
                number: generation,
                timestamp: self.time.elapsed_secs_as_float(),
                i_all_ratio: self.improvement_tracker.i_all_ratio,
                i_1000_ratio: self.improvement_tracker.i_1000_ratio,
                is_improvement: self.improvement_tracker.is_last_improved,
                population: TelemetryPopulation { individuals },
            });
        }
    }

    /// Reports final statistic.
    pub fn on_result(&mut self, population: &DynHeuristicPopulation<O, S>) {
        let generations = self.statistics.generation;

        let (should_log_population, should_track_population) = match &self.mode {
            TelemetryMode::OnlyLogging { .. } => (true, false),
            TelemetryMode::OnlyMetrics { track_population, .. } => (false, generations % track_population != 0),
            TelemetryMode::All { track_population, .. } => (true, generations % track_population != 0),
            _ => return,
        };

        self.on_population(population, should_log_population, should_track_population);

        let elapsed = self.time.elapsed_secs() as usize;
        let speed = generations as Float / self.time.elapsed_secs_as_float();

        self.log(format!("[{elapsed}s] total generations: {generations}, speed: {speed:.2} gen/sec",).as_str());
        match population.ranked().next() {
            Some(best) => {
                self.log(format!("\tbest fitness: ({})", format_fitness(best.fitness())).as_str());
            }
            _ => {
                self.log("no solutions found");
            }
        }

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
            TelemetryMode::OnlyLogging { logger, .. } => (logger)(message),
            TelemetryMode::All { logger, .. } => (logger)(message),
            _ => {}
        }
    }

    /// Returns current statistics.
    pub fn get_statistics(&self) -> &HeuristicStatistics {
        &self.statistics
    }

    fn get_individual_metrics(&self, population: &DynHeuristicPopulation<O, S>, solution: &S) -> TelemetryIndividual {
        let fitness = solution.fitness().collect::<Vec<_>>();

        let difference = get_fitness_change(population, solution);

        TelemetryIndividual { difference, fitness }
    }

    fn log_individual(&self, metrics: &TelemetryIndividual, gen_info: Option<(usize, Timer)>) {
        let fitness = format_fitness(metrics.fitness.iter().cloned());

        let value = if let Some((r#gen, gen_time)) = gen_info {
            format!(
                "[{}s] generation {} took {}ms, median: {}ms fitness: ({})",
                self.time.elapsed_secs(),
                r#gen,
                gen_time.elapsed_millis(),
                self.speed_tracker.median.approx_median().unwrap_or(0),
                fitness
            )
        } else {
            format!("\tfitness: ({}), difference: {:.3}%", fitness, metrics.difference)
        };

        self.log(value.as_str());
    }
}

struct ImprovementTracker {
    buffer: Vec<bool>,
    total_improvements: usize,

    pub i_all_ratio: Float,
    pub i_1000_ratio: Float,
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

        self.i_all_ratio = (self.total_improvements as Float) / ((generation + 1) as Float);
        self.i_1000_ratio = (improvements as Float) / ((generation + 1).min(self.buffer.len()) as Float);
    }
}

struct SpeedTracker {
    initial_estimate: Float,
    initial_time: Float,
    last_time: Float,
    median: RemedianUsize,
    speed: HeuristicSpeed,
}

impl Default for SpeedTracker {
    fn default() -> Self {
        Self {
            initial_estimate: 0.,
            initial_time: 0.,
            last_time: 0.,
            median: RemedianUsize::new(11, 7, |a, b| a.cmp(b)),
            speed: HeuristicSpeed::Unknown,
        }
    }
}

impl SpeedTracker {
    pub fn track(&mut self, generation: usize, time: &Timer, termination_estimate: Float) {
        let elapsed = (time.elapsed_millis() as Float) * 1000.;
        if generation == 0 {
            self.initial_estimate = termination_estimate;
            self.initial_time = elapsed;
            self.last_time = elapsed;
        } else {
            let duration = ((elapsed - self.last_time) / 1000.).round() as usize;
            self.median.add_observation(duration);
            self.last_time = elapsed;

            // average gen/sec speed excluding initial solutions
            let average = if elapsed > self.initial_time {
                generation as Float / ((elapsed - self.initial_time) / 1_000_000.)
            } else {
                1000.
            };

            let delta = (termination_estimate - self.initial_estimate).max(0.);

            let ratio = match (generation, delta, average) {
                (generation, delta, _) if generation < 10 && delta > 0.1 => 0.1,
                (generation, delta, _) if generation < 100 && delta > 0.1 => 0.25,
                (generation, delta, _) if generation < 200 && delta > 0.1 => 0.5,
                (generation, delta, _) if generation < 500 && delta > 0.2 => 0.5,

                (generation, _, average) if generation > 5 && average < 4. => 0.25,
                (generation, _, average) if generation > 5 && average < 8. => 0.5,
                _ => 1.,
            };

            let is_slow = ratio < 1.;
            let median = self.median.approx_median();

            self.speed = match &self.speed {
                HeuristicSpeed::Unknown | HeuristicSpeed::Moderate { .. } if !is_slow => {
                    HeuristicSpeed::Moderate { average, median }
                }
                _ => HeuristicSpeed::Slow { ratio, average, median },
            }
        }
    }

    pub fn get_current_speed(&self) -> HeuristicSpeed {
        self.speed.clone()
    }
}

fn get_fitness_change<O, S>(population: &DynHeuristicPopulation<O, S>, solution: &S) -> Float
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let fitness_change = population
        .ranked()
        .next()
        .map(|best_ctx| best_ctx.fitness())
        .map(|best_fitness| {
            let fitness_value = solution.fitness();
            relative_distance(fitness_value, best_fitness)
        })
        .unwrap_or(0.);

    fitness_change
}

fn format_fitness(fitness: impl Iterator<Item = Float>) -> String {
    fitness.map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(", ")
}
