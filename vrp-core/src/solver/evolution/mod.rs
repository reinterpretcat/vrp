use crate::construction::heuristics::InsertionContext;
use crate::solver::telemetry::Telemetry;
use crate::solver::{Metrics, RefinementContext, TargetHeuristic, TargetPopulation, TargetTermination};
use rosomaxa::prelude::*;
use rosomaxa::utils::Timer;

pub use self::config::*;
pub use self::run_simple::RunSimple;

#[cfg(test)]
#[path = "../../../tests/unit/solver/evolution/evolution_test.rs"]
mod evolution_test;

mod config;
mod run_simple;

/// Defines evolution result type.
pub type EvolutionResult = Result<(TargetPopulation, Option<Metrics>), String>;

/// An evolution algorithm strategy.
pub trait EvolutionStrategy {
    /// Runs evolution for given `refinement_ctx`.
    fn run(
        &self,
        refinement_ctx: RefinementContext,
        heuristic: TargetHeuristic,
        termination: &TargetTermination,
        telemetry: Telemetry,
    ) -> EvolutionResult;
}

/// An entity which simulates evolution process.
pub struct EvolutionSimulator {
    config: EvolutionConfig,
}

impl EvolutionSimulator {
    pub fn new(config: EvolutionConfig) -> Result<Self, String> {
        if config.population.initial.methods.is_empty() {
            return Err("at least one initial method has to be specified".to_string());
        }

        Ok(Self { config })
    }

    /// Runs evolution for given `problem` using evolution `config`.
    /// Returns populations filled with solutions.
    pub fn run(mut self) -> EvolutionResult {
        let refinement_ctx = self.create_refinement_ctx();
        let strategy = self.config.strategy.clone();

        strategy.run(refinement_ctx, self.config.heuristic, self.config.termination.as_ref(), self.config.telemetry)
    }

    /// Creates refinement context with population containing initial individuals.
    fn create_refinement_ctx(&mut self) -> RefinementContext {
        let mut refinement_ctx = RefinementContext::new(
            self.config.problem.clone(),
            std::mem::replace(&mut self.config.population.population, None).unwrap(),
            self.config.environment.clone(),
            std::mem::replace(&mut self.config.quota, None),
        );

        self.config.telemetry.log(
            format!(
                "problem has total jobs: {}, actors: {}",
                self.config.problem.jobs.size(),
                self.config.problem.fleet.actors.len()
            )
            .as_str(),
        );

        self.config.telemetry.log("preparing initial solution(-s)");

        std::mem::take(&mut self.config.population.initial.individuals)
            .into_iter()
            .zip(0_usize..)
            .take(self.config.population.initial.max_size)
            .for_each(|(ctx, idx)| {
                if should_add_solution(&refinement_ctx) {
                    self.config.telemetry.on_initial(
                        &ctx,
                        idx,
                        self.config.population.initial.max_size,
                        Timer::start(),
                        self.config.termination.estimate(&refinement_ctx),
                    );
                    refinement_ctx.population.add(ctx);
                } else {
                    self.config.telemetry.log(format!("skipping provided initial solution {}", idx).as_str())
                }
            });

        let weights = self.config.population.initial.methods.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
        let empty_ctx = InsertionContext::new(self.config.problem.clone(), refinement_ctx.environment.clone());

        let initial_time = Timer::start();
        let _ = (refinement_ctx.population.size()..self.config.population.initial.max_size).try_for_each(|idx| {
            let item_time = Timer::start();

            let is_overall_termination = self.config.termination.is_termination(&mut refinement_ctx);
            let is_initial_quota_reached =
                self.config.termination.estimate(&refinement_ctx) > self.config.population.initial.quota;

            if is_initial_quota_reached || is_overall_termination {
                self.config.telemetry.log(
                    format!(
                        "stop building initial solutions due to initial quota reached ({}) or overall termination ({}).",
                        is_initial_quota_reached, is_overall_termination
                    )
                        .as_str(),
                );
                return Err(());
            }

            let method_idx = if idx < self.config.population.initial.methods.len() {
                idx
            } else {
                self.config.environment.random.weighted(weights.as_slice())
            };

            // TODO consider initial quota limit
            let insertion_ctx = self.config.population.initial.methods[method_idx].0.run(&refinement_ctx, empty_ctx.deep_copy());

            if should_add_solution(&refinement_ctx) {
                self.config.telemetry.on_initial(
                    &insertion_ctx,
                    idx,
                                                 self.config.population.initial.max_size, item_time,
                                                 self.config.termination.estimate(&refinement_ctx));
                refinement_ctx.population.add(insertion_ctx);
            } else {
                self.config.telemetry.log(format!("skipping built initial solution {}", idx).as_str())
            }

            Ok(())
        });

        if refinement_ctx.population.size() > 0 {
            on_generation(
                &mut refinement_ctx,
                &mut self.config.telemetry,
                self.config.termination.as_ref(),
                initial_time,
                true,
            );
        } else {
            self.config.telemetry.log("created an empty population");
        }

        refinement_ctx
    }
}

fn should_add_solution(refinement_ctx: &RefinementContext) -> bool {
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |quota| quota.is_reached());
    let is_population_empty = refinement_ctx.population.size() == 0;

    // NOTE when interrupted, population can return solution with worse primary objective fitness values as first
    is_population_empty || !is_quota_reached
}

fn should_stop(refinement_ctx: &mut RefinementContext, termination: &TargetTermination) -> bool {
    let is_terminated = termination.is_termination(refinement_ctx);
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |q| q.is_reached());

    is_terminated || is_quota_reached
}

fn on_generation(
    refinement_ctx: &mut RefinementContext,
    telemetry: &mut Telemetry,
    termination: &TargetTermination,
    generation_time: Timer,
    is_improved: bool,
) {
    let termination_estimate = termination.estimate(refinement_ctx);

    telemetry.on_generation(refinement_ctx, termination_estimate, generation_time, is_improved);
    refinement_ctx.population.on_generation(&refinement_ctx.statistics);
}
