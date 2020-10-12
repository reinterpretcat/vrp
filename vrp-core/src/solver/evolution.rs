#[cfg(test)]
#[path = "../../tests/unit/solver/evolution_test.rs"]
mod evolution_test;

use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::Problem;
use crate::solver::mutation::*;
use crate::solver::population::DominancePopulation;
use crate::solver::selection::{NaiveSelection, Selection};
use crate::solver::telemetry::Telemetry;
use crate::solver::termination::*;
use crate::solver::{Metrics, Population, RefinementContext, TelemetryMode};
use crate::utils::{get_cpus, DefaultRandom, Random, Timer};
use std::sync::Arc;

/// A configuration which controls evolution execution.
pub struct EvolutionConfig {
    /// An original problem.
    pub problem: Arc<Problem>,
    /// A selection defines parents to be selected on each generation.
    pub selection: Arc<dyn Selection>,
    /// A mutation applied to population.
    pub mutation: Arc<dyn Mutation + Send + Sync>,
    /// A termination defines when evolution should stop.
    pub termination: Arc<dyn Termination>,
    /// A quota for evolution execution.
    pub quota: Option<Arc<dyn Quota + Send + Sync>>,
    /// A population configuration
    pub population: PopulationConfig,
    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
    /// A telemetry to be used.
    pub telemetry: Telemetry,
}

/// Contains population specific properties.
pub struct PopulationConfig {
    /// An initial solution config.
    pub initial: InitialConfig,
    /// Max population size.
    pub max_size: usize,
}

/// An initial solutions configuration.
pub struct InitialConfig {
    /// Initial size of population to be generated.
    pub size: usize,
    /// Create methods to produce initial individuals.
    pub methods: Vec<(Box<dyn Recreate + Send + Sync>, usize)>,
    /// Initial individuals in population.
    pub individuals: Vec<InsertionContext>,
}

impl EvolutionConfig {
    pub fn new(problem: Arc<Problem>) -> Self {
        Self {
            problem: problem.clone(),
            selection: Arc::new(NaiveSelection::new(get_cpus())),
            mutation: Arc::new(CompositeMutation::new(vec![(
                Arc::new(RuinAndRecreate::new_from_problem(
                    problem,
                    (Box::new(CompositeLocalSearch::default()), 0.05),
                    (Box::new(CompositeLocalSearch::default()), 0.05),
                )),
                100,
            )])),
            termination: Arc::new(CompositeTermination::new(vec![
                Box::new(MaxTime::new(300.)),
                Box::new(MaxGeneration::new(3000)),
            ])),
            quota: None,
            random: Arc::new(DefaultRandom::default()),
            telemetry: Telemetry::new(TelemetryMode::None),
            population: PopulationConfig {
                max_size: 2,
                initial: InitialConfig {
                    size: 1,
                    methods: vec![(Box::new(RecreateWithCheapest::default()), 10)],
                    individuals: vec![],
                },
            },
        }
    }
}

/// An entity which simulates evolution process.
pub struct EvolutionSimulator {
    config: EvolutionConfig,
}

impl EvolutionSimulator {
    pub fn new(config: EvolutionConfig) -> Result<Self, String> {
        if config.population.initial.size < 1 {
            return Err("initial size should be greater than 0".to_string());
        }

        if config.population.initial.size > config.population.max_size {
            return Err("initial size should be less or equal population size".to_string());
        }

        if config.population.initial.methods.is_empty() {
            return Err("at least one initial method has to be specified".to_string());
        }

        Ok(Self { config })
    }

    /// Runs evolution for given `problem` using evolution `config`.
    /// Returns populations filled with solutions.
    pub fn run(mut self) -> Result<(Box<dyn Population>, Option<Metrics>), String> {
        self.config.telemetry.start();

        let mut refinement_ctx = self.create_refinement_ctx()?;

        while !self.should_stop(&mut refinement_ctx) {
            let generation_time = Timer::start();

            let parents = self.config.selection.select_parents(&refinement_ctx);

            let offspring = self.config.mutation.mutate_all(&refinement_ctx, parents);

            let is_improved =
                if should_add_solution(&refinement_ctx) { refinement_ctx.population.add_all(offspring) } else { false };

            self.config.telemetry.on_generation(&mut refinement_ctx, generation_time, is_improved);
        }

        self.config.telemetry.on_result(&refinement_ctx);

        Ok((refinement_ctx.population, self.config.telemetry.get_metrics()))
    }

    /// Creates refinement context with population containing initial individuals.
    fn create_refinement_ctx(&mut self) -> Result<RefinementContext, String> {
        let mut refinement_ctx = RefinementContext::new(
            self.config.problem.clone(),
            Box::new(DominancePopulation::new(self.config.problem.clone(), self.config.population.max_size)),
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

        std::mem::replace(&mut self.config.population.initial.individuals, vec![])
            .into_iter()
            .zip(0_usize..)
            .take(self.config.population.initial.size)
            .for_each(|(ctx, idx)| {
                if should_add_solution(&refinement_ctx) {
                    self.config.telemetry.on_initial(idx, self.config.population.initial.size, Timer::start());
                    refinement_ctx.population.add(ctx);
                } else {
                    self.config.telemetry.log(format!("skipping provided initial solution {}", idx).as_str())
                }
            });

        let weights = self.config.population.initial.methods.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
        let empty_ctx = InsertionContext::new(self.config.problem.clone(), self.config.random.clone());

        let initial_time = Timer::start();
        let _ = (refinement_ctx.population.size()..self.config.population.initial.size).try_for_each(|idx| {
            let item_time = Timer::start();

            if self.config.termination.is_termination(&mut refinement_ctx) {
                return Err(());
            }

            let method_idx = self.config.random.weighted(weights.as_slice());

            let insertion_ctx =
                self.config.population.initial.methods[method_idx].0.run(&refinement_ctx, empty_ctx.deep_copy());

            if should_add_solution(&refinement_ctx) {
                refinement_ctx.population.add(insertion_ctx);
                self.config.telemetry.on_initial(idx, self.config.population.initial.size, item_time);
            } else {
                self.config.telemetry.log(format!("skipping built initial solution {}", idx).as_str())
            }

            Ok(())
        });

        if refinement_ctx.population.size() > 0 {
            self.config.telemetry.on_generation(&mut refinement_ctx, initial_time, true);
        } else {
            self.config.telemetry.log("created an empty population")
        }

        Ok(refinement_ctx)
    }

    fn should_stop(&self, refinement_ctx: &mut RefinementContext) -> bool {
        let is_terminated = self.config.termination.is_termination(refinement_ctx);
        let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |q| q.is_reached());

        is_terminated || is_quota_reached
    }
}

fn should_add_solution(refinement_ctx: &RefinementContext) -> bool {
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |quota| quota.is_reached());
    let is_population_empty = refinement_ctx.population.size() == 0;

    // NOTE when interrupted, population can return solution with worse primary objective fitness values as first
    is_population_empty || !is_quota_reached
}
