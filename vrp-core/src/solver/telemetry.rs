//! A module which provides the logic to collect metrics about algorithm execution and simple logging.

use crate::algorithms::nsga2::{MultiObjective, Objective};
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::utils::Timer;
use std::ops::Deref;
use std::sync::Arc;

/// A logger type which is called with various information regarding the work done by the VRP solver.
pub type InfoLogger = Arc<dyn Fn(&str) -> ()>;

/// Encapsulates different measurements regarding algorithm evaluation.
pub struct Metrics {
    /// Timestamp when algorithm is started.
    pub timestamp: usize,
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
    /// Population state.
    pub population: Vec<Individual>,
}

/// Keeps essential information about particular individual in population.
pub struct Individual {
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

/// Specifies a telemetry mode.
pub enum TelemetryMode {
    /// No telemetry at all.
    None,
    /// Only logging to specified info logger.
    OnlyLogging { logger: InfoLogger, log_best: usize, log_population: usize },
    /// Only metrics collection.
    OnlyMetrics { track_population: usize },
    /// Both logging and metrics collection.
    All { logger: InfoLogger, log_best: usize, log_population: usize, track_population: usize },
}

/// Provides way to collect metrics and write information into log.
pub struct Telemetry {
    metrics: Metrics,
    time: Timer,
    mode: TelemetryMode,
}

impl Telemetry {
    pub fn new(mode: TelemetryMode) -> Self {
        Self {
            time: Timer::start(),
            metrics: Metrics { timestamp: 0, duration: 0, generations: 0, speed: 0.0, evolution: vec![] },
            mode,
        }
    }

    pub fn start(&mut self) {
        self.time = Timer::start();
    }

    pub fn on_initial(&mut self, item_idx: usize, total_items: usize, item_time: Timer) {
        match &self.mode {
            TelemetryMode::OnlyLogging { .. } | TelemetryMode::All { .. } => self.log(
                format!(
                    "[{}s] created {} of {} initial solutions in {}ms",
                    self.time.elapsed_secs(),
                    item_idx + 1,
                    total_items,
                    item_time.elapsed_millis()
                )
                .as_str(),
            ),
            _ => {}
        };
    }

    pub fn on_progress(&mut self, refinement_ctx: &RefinementContext, generation_time: Timer) {
        let (log_best, log_population, track_population) = match &self.mode {
            TelemetryMode::None => return,
            TelemetryMode::OnlyLogging { log_best, log_population, .. } => (Some(log_best), Some(log_population), None),
            TelemetryMode::OnlyMetrics { track_population, .. } => (None, None, Some(track_population)),
            TelemetryMode::All { log_best, log_population, track_population, .. } => {
                (Some(log_best), Some(log_population), Some(track_population))
            }
        };

        if let Some(best_individual) = refinement_ctx.population.best() {
            let generation = refinement_ctx.generation;
            let should_log_best = generation % *log_best.unwrap_or(&usize::MAX) == 0;
            let should_log_population = generation % *log_population.unwrap_or(&usize::MAX) == 0 || generation == 1;
            let should_track_population = generation % *track_population.unwrap_or(&usize::MAX) == 0 || generation == 1;

            if should_log_best {
                self.log_individual(
                    &self.get_individual_metrics(refinement_ctx, &best_individual),
                    Some((refinement_ctx.generation, generation_time)),
                )
            }

            self.on_population(&refinement_ctx, should_log_population, should_track_population);
        } else {
            self.log("no progress yet");
        }
    }

    pub fn on_population(
        &mut self,
        refinement_ctx: &RefinementContext,
        should_log_population: bool,
        should_track_population: bool,
    ) {
        if !should_log_population && !should_track_population {
            return;
        }

        if should_log_population {
            self.log(
                format!(
                    "[{}s] population state (speed: {:.2} gen/sec):",
                    self.time.elapsed_secs(),
                    refinement_ctx.generation as f64 / self.time.elapsed_secs_as_f64(),
                )
                .as_str(),
            );
        }

        let population_metrics = refinement_ctx
            .population
            .all()
            .map(|insertion_ctx| self.get_individual_metrics(refinement_ctx, &insertion_ctx))
            .collect::<Vec<_>>();

        if should_log_population {
            population_metrics.iter().for_each(|metrics| self.log_individual(&metrics, None))
        }

        if should_track_population {
            self.metrics.evolution.push(Generation {
                number: refinement_ctx.generation,
                timestamp: self.time.elapsed_secs_as_f64() ,
                population: population_metrics,
            })
        }
    }

    pub fn on_result(&mut self, refinement_ctx: &RefinementContext) {
        let should_log_population = match &self.mode {
            TelemetryMode::OnlyLogging { .. } => true,
            TelemetryMode::OnlyMetrics { .. } => false,
            TelemetryMode::All { .. } => true,
            _ => return,
        };

        self.on_population(refinement_ctx, should_log_population, false);

        let elapsed = self.time.elapsed_secs() as usize;
        let speed = refinement_ctx.generation as f64 / self.time.elapsed_secs_as_f64();

        self.log(
            format!("[{}s] total generations: {}, speed: {:.2} gen/sec", elapsed, refinement_ctx.generation, speed)
                .as_str(),
        );

        self.metrics.duration = elapsed;
        self.metrics.generations = refinement_ctx.generation;
        self.metrics.speed = speed;
    }

    pub fn get_metrics(self) -> Option<Metrics> {
        match &self.mode {
            TelemetryMode::OnlyMetrics { .. } | TelemetryMode::All { .. } => Some(self.metrics),
            _ => None,
        }
    }

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
    ) -> Individual {
        let fitness_values = insertion_ctx
            .problem
            .objective
            .objectives()
            .map(|objective| objective.fitness(insertion_ctx))
            .collect::<Vec<_>>();

        let (cost, cost_difference) = Self::get_fitness(refinement_ctx, insertion_ctx);

        Individual {
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
                "{}cost: {:.2}({:.3}%), tours: {}, unassigned: {}, fitness: ({})",
                gen_info.map_or("\t".to_string(), |(gen, gen_time)| format!(
                    "[{}s] generation {} took {}ms, ",
                    self.time.elapsed_secs(),
                    gen,
                    gen_time.elapsed_millis()
                )),
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
            .best()
            .map(|best_ctx| refinement_ctx.problem.objective.fitness(best_ctx))
            .map(|best_fitness| (fitness_value - best_fitness) / best_fitness * 100.)
            .unwrap_or(0.);

        (fitness_value, fitness_change)
    }
}
