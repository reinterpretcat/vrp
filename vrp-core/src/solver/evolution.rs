use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::Problem;
use crate::solver::mutation::{Mutation, Recreate};
use crate::solver::population::DominancePopulation;
use crate::solver::telemetry::Telemetry;
use crate::solver::termination::Termination;
use crate::solver::{Metrics, Population, RefinementContext};
use crate::utils::{parallel_into_collect, Random, Timer};
use std::cmp::Ordering;
use std::iter::once;
use std::ops::Range;
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
    /// A population configuration
    pub population: PopulationConfig,
    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
    /// A telemetry to be used.
    pub telemetry: Telemetry,
}

/// Contains population specific properties.
pub struct PopulationConfig {
    /// Max population size.
    pub size: usize,
    /// An initial solution config.
    pub initial: InitialConfig,
    /// An offspring config.
    pub offspring: OffspringConfig,
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

/// An offspring configuration.
pub struct OffspringConfig {
    /// Offspring size.
    pub size: usize,
    /// A chance to have a branch as (normal probability, intensive probability, improvement threshold).
    pub chance: (f64, f64, f64),
    /// A range of generations in branch.
    pub generations: Range<usize>,
    /// An acceptance curve steepness used in formula `1 - (x/total_generations)^steepness`.
    pub steepness: f64,
}

/// An entity which simulates evolution process.
pub struct EvolutionSimulator {
    problem: Arc<Problem>,
    config: EvolutionConfig,
}

impl EvolutionSimulator {
    pub fn new(problem: Arc<Problem>, config: EvolutionConfig) -> Result<Self, String> {
        if config.population.initial.size < 1 {
            return Err("initial size should be greater than 0".to_string());
        }

        if config.population.initial.size > config.population.size {
            return Err("initial size should be less or equal population size".to_string());
        }

        if config.population.initial.methods.is_empty() {
            return Err("at least one initial method has to be specified".to_string());
        }

        if config.population.offspring.size < 1 {
            return Err("offspring size should be greater than 0".to_string());
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
            let offspring_cfg = &self.config.population.offspring;
            let branching_chance = self.get_branching_chance();

            let offspring = parallel_into_collect(parents, |ctx| {
                let is_branch = ctx.random.uniform_real(0., 1.) < branching_chance;
                if is_branch {
                    let random = ctx.random.clone();
                    let (min, max) = (offspring_cfg.generations.start as i32, offspring_cfg.generations.end as i32);
                    let gens = random.uniform_int(min, max) as usize;
                    (1_usize..=gens).fold(ctx, |parent, idx| {
                        let child = mutator.mutate(&refinement_ctx, parent.deep_copy());

                        let use_worse_chance = random.uniform_real(0., 1.);
                        let use_worse_probability = get_use_worse_probability(idx, gens, offspring_cfg.steepness);
                        let is_child_better = refinement_ctx.population.cmp(&child, &parent) == Ordering::Less;

                        if use_worse_chance < use_worse_probability || is_child_better {
                            child
                        } else {
                            parent
                        }
                    })
                } else {
                    mutator.mutate(&refinement_ctx, ctx)
                }
            });

            let is_improved =
                if should_add_solution(&refinement_ctx) { refinement_ctx.population.add_all(offspring) } else { false };

            self.config.telemetry.on_generation(&refinement_ctx, generation_time, is_improved);

            refinement_ctx.generation += 1;
        }

        self.config.telemetry.on_result(&refinement_ctx);

        Ok((refinement_ctx.population, self.config.telemetry.get_metrics()))
    }

    /// Creates refinement context with population containing initial individuals.
    fn create_refinement_ctx(&mut self) -> Result<RefinementContext, String> {
        let mut refinement_ctx = RefinementContext::new(
            self.problem.clone(),
            Box::new(DominancePopulation::new(self.problem.clone(), self.config.population.size)),
            std::mem::replace(&mut self.config.quota, None),
        );

        std::mem::replace(&mut self.config.population.initial.individuals, vec![])
            .into_iter()
            .zip(0_usize..)
            .take(self.config.population.initial.size)
            .for_each(|(ctx, idx)| {
                self.config.telemetry.on_initial(idx, self.config.population.initial.size, Timer::start());
                refinement_ctx.population.add(ctx);
            });

        let weights = self.config.population.initial.methods.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
        let empty_ctx = InsertionContext::new(self.problem.clone(), self.config.random.clone());

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
            }

            self.config.telemetry.on_initial(idx, self.config.population.initial.size, item_time);

            Ok(())
        });

        Ok(refinement_ctx)
    }

    fn select_parents(&self, refinement_ctx: &RefinementContext) -> Vec<InsertionContext> {
        assert!(refinement_ctx.population.size() > 0);

        once(0_usize)
            .chain(
                (1..self.config.population.offspring.size)
                    .map(|_| self.config.random.uniform_int(0, refinement_ctx.population.size() as i32 - 1) as usize),
            )
            .take(self.config.population.offspring.size)
            .filter_map(|idx| refinement_ctx.population.nth(idx))
            .map(|individual| individual.deep_copy())
            .collect()
    }

    fn get_branching_chance(&self) -> f64 {
        let (normal, intensive, threshold) = self.config.population.offspring.chance;
        if self.config.telemetry.get_improvement_ratio().1 < threshold {
            intensive
        } else {
            normal
        }
    }
}

fn should_add_solution(refinement_ctx: &RefinementContext) -> bool {
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |quota| quota.is_reached());
    let is_population_empty = refinement_ctx.population.size() == 0;

    // NOTE when interrupted, population can return solution with worse primary objective fitness values as first
    is_population_empty || !is_quota_reached
}

fn get_use_worse_probability(current: usize, total: usize, steepness: f64) -> f64 {
    1. - (current as f64 / total as f64).powf(steepness)
}
