use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::Problem;
use crate::solver::mutation::{Mutation, Recreate};
use crate::solver::population::DominancePopulation;
use crate::solver::telemetry::Telemetry;
use crate::solver::termination::Termination;
use crate::solver::{Metrics, Population, RefinementContext};
use crate::utils::{parallel_into_collect, Random, Timer};
use hashbrown::HashSet;
use std::sync::Arc;

/// A configuration which controls evolution execution.
pub struct EvolutionConfig {
    /// An original problem.
    pub problem: Arc<Problem>,
    /// A mutation applied to population.
    pub mutation: Box<dyn Mutation + Send + Sync>,
    /// A termination defines when evolution should stop.
    pub termination: Box<dyn Termination>,
    /// A quota for evolution execution.
    pub quota: Option<Box<dyn Quota + Send + Sync>>,

    /// Population size.
    pub population_size: usize,
    /// Offspring size.
    pub offspring_size: usize,
    /// Initial size of population to be generated.
    pub initial_size: usize,
    /// Create methods to produce initial individuals.
    pub initial_methods: Vec<(Box<dyn Recreate + Send + Sync>, usize)>,
    /// Initial individuals in population.
    pub initial_individuals: Vec<InsertionContext>,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
    /// A telemetry to be used.
    pub telemetry: Telemetry,
}

/// An entity which simulates evolution process.
pub struct EvolutionSimulator {
    problem: Arc<Problem>,
    config: EvolutionConfig,
}

impl EvolutionSimulator {
    pub fn new(problem: Arc<Problem>, config: EvolutionConfig) -> Result<Self, String> {
        if config.initial_size < 1 {
            return Err("initial size should be greater than 0".to_string());
        }

        if config.initial_size > config.population_size {
            return Err("initial size should be less or equal population size".to_string());
        }

        if config.initial_methods.is_empty() {
            return Err("at least one initial method has to be specified".to_string());
        }

        Ok(Self { problem, config })
    }

    /// Runs evolution for given `problem` using evolution `config`.
    /// Returns populations filled with solutions.
    pub fn run(mut self) -> Result<(Box<dyn Population>, Option<Metrics>), String> {
        self.config.telemetry.start();

        let mut refinement_ctx = self.create_refinement_ctx()?;

        // NOTE at the moment, only one solution is produced per generation
        while !self.config.termination.is_termination(&mut refinement_ctx) {
            let generation_time = Timer::start();

            let mutator = &self.config.mutation;
            let parents = self.select_parents(&refinement_ctx);
            let offspring = parallel_into_collect(parents, |ctx| mutator.mutate(&refinement_ctx, ctx));

            refinement_ctx.population.add_all(offspring);

            self.config.telemetry.on_progress(&refinement_ctx, generation_time);

            refinement_ctx.generation += 1;
        }

        self.config.telemetry.on_result(&refinement_ctx);

        Ok((refinement_ctx.population, self.config.telemetry.get_metrics()))
    }

    /// Creates refinement context with population containing initial individuals.
    fn create_refinement_ctx(&mut self) -> Result<RefinementContext, String> {
        let mut refinement_ctx = RefinementContext::new(
            self.problem.clone(),
            Box::new(DominancePopulation::new(self.problem.clone(), self.config.population_size)),
            std::mem::replace(&mut self.config.quota, None),
        );

        std::mem::replace(&mut self.config.initial_individuals, vec![])
            .into_iter()
            .take(self.config.initial_size)
            .for_each(|ctx| refinement_ctx.population.add(ctx));

        let weights = self.config.initial_methods.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
        let empty_ctx = InsertionContext::new(self.problem.clone(), self.config.random.clone());

        let indices: Vec<_> = if self.config.initial_size <= self.config.initial_methods.len() {
            (0..self.config.initial_size).collect()
        } else {
            (refinement_ctx.population.size()..self.config.initial_size)
                .map(|_| self.config.random.weighted(weights.as_slice()))
                .collect()
        };

        let _ = indices.into_iter().enumerate().try_for_each(|(idx, method_idx)| {
            let item_time = Timer::start();

            if self.config.termination.is_termination(&mut refinement_ctx) {
                return Err(());
            }

            let insertion_ctx = self.config.initial_methods[method_idx].0.run(&refinement_ctx, empty_ctx.deep_copy());

            if should_add_solution(&refinement_ctx) {
                refinement_ctx.population.add(insertion_ctx);
            }

            self.config.telemetry.on_initial(idx, self.config.initial_size, item_time);

            Ok(())
        });

        Ok(refinement_ctx)
    }

    fn select_parents(&self, refinement_ctx: &RefinementContext) -> Vec<InsertionContext> {
        let pareto_opt = refinement_ctx
            .population
            .ranked()
            .filter_map(|(insertion_ctx, rank)| if rank == 0 { Some(insertion_ctx) } else { None })
            .take(self.config.offspring_size)
            .collect::<Vec<_>>();

        let extra_parents_size = self.config.offspring_size - pareto_opt.len();

        if extra_parents_size == 0 {
            return pareto_opt.iter().map(|insertion_ctx| insertion_ctx.deep_copy()).collect();
        }

        // NOTE use uniform distribution, there might be better alternatives
        let extra_parents_indices = (0..extra_parents_size)
            .map(|_| self.config.random.uniform_int(0, extra_parents_size as i32 - 1) as usize)
            .collect::<HashSet<_>>();

        let skip = pareto_opt.len();
        pareto_opt
            .into_iter()
            .chain(refinement_ctx.population.ranked().skip(skip).enumerate().filter_map(|(idx, (insertion_ctx, _))| {
                if extra_parents_indices.contains(&idx) {
                    Some(insertion_ctx)
                } else {
                    None
                }
            }))
            .map(|insertion_ctx| insertion_ctx.deep_copy())
            .collect()
    }
}

fn should_add_solution(refinement_ctx: &RefinementContext) -> bool {
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |quota| quota.is_reached());
    let is_population_empty = refinement_ctx.population.size() == 0;

    // TODO fix population not to accept solution with worse primary objective fitness as best
    is_population_empty || !is_quota_reached
}
