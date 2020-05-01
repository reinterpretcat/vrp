use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::common::{MultiObjective, Objective};
use crate::models::Problem;
use crate::solver::mutation::{Mutation, Recreate};
use crate::solver::population::DominancePopulation;
use crate::solver::termination::Termination;
use crate::solver::Logger;
use crate::solver::{Population, RefinementContext};
use crate::utils::{Random, Timer};
use std::ops::Deref;
use std::sync::Arc;

/// A configuration which controls evolution execution.
pub struct EvolutionConfig {
    /// A mutation applied to population.
    pub mutation: Box<dyn Mutation>,
    /// A termination defines when evolution should stop.
    pub termination: Box<dyn Termination>,
    /// A quota for evolution execution.
    pub quota: Option<Box<dyn Quota + Send + Sync>>,

    /// Population size.
    pub population_size: usize,
    /// Offspring size.
    pub offspring_size: usize,
    /// Elite size.
    pub elite_size: usize,
    /// Initial size of population to be generated.
    pub initial_size: usize,
    /// Create methods to create initial individuals.
    pub initial_methods: Vec<(Box<dyn Recreate>, usize)>,
    /// Initial individuals in population.
    pub initial_individuals: Vec<InsertionContext>,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
    /// A logger used to log evolution progress.
    pub logger: Logger,
}

/// Runs evolution for given `problem` using evolution `config`.
/// Returns populations filled with solutions.
pub fn run_evolution(problem: Arc<Problem>, config: EvolutionConfig) -> Result<Box<dyn Population>, String> {
    let mut config = config;

    let evolution_time = Timer::start();

    let mut refinement_ctx = create_refinement_ctx(problem.clone(), &mut config, &evolution_time)?;

    // NOTE at the moment, only one solution is produced per generation
    while !config.termination.is_termination(&mut refinement_ctx) {
        let generation_time = Timer::start();

        let insertion_ctx = refinement_ctx.population.select().deep_copy();

        let insertion_ctx = config.mutation.mutate(&mut refinement_ctx, insertion_ctx);

        log_progress(&refinement_ctx, &evolution_time, Some(&generation_time), &config.logger);

        add_solution(&mut refinement_ctx, insertion_ctx);

        refinement_ctx.generation += 1;
    }

    log_result(&refinement_ctx, &evolution_time, &config.logger);

    Ok(refinement_ctx.population)
}

/// Creates refinement context with population containing initial individuals.
fn create_refinement_ctx(
    problem: Arc<Problem>,
    config: &mut EvolutionConfig,
    evolution_time: &Timer,
) -> Result<RefinementContext, String> {
    if config.initial_size < 1 {
        return Err("initial size should be greater than 0".to_string());
    }

    if config.initial_size > config.population_size {
        return Err("initial size should be less or equal population size".to_string());
    }

    if config.initial_methods.len() < 1 {
        return Err("at least one initial method has to be specified".to_string());
    }

    let mut refinement_ctx = RefinementContext::new(
        problem.clone(),
        Box::new(DominancePopulation::new(
            problem.clone(),
            config.random.clone(),
            config.population_size,
            config.offspring_size,
            config.elite_size,
        )),
        std::mem::replace(&mut config.quota, None),
    );

    std::mem::replace(&mut config.initial_individuals, vec![])
        .into_iter()
        .take(config.initial_size)
        .for_each(|ctx| refinement_ctx.population.add(ctx));

    let weights = config.initial_methods.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
    let empty_ctx = InsertionContext::new(problem.clone(), config.random.clone());

    let indices: Vec<_> = if config.initial_size <= config.initial_methods.len() {
        (0..config.initial_size).collect()
    } else {
        (refinement_ctx.population.size()..config.initial_size)
            .map(|_| config.random.weighted(weights.as_slice()))
            .collect()
    };

    let _ = indices.into_iter().enumerate().try_for_each(|(idx, method_idx)| {
        let item_time = Timer::start();

        if config.termination.is_termination(&mut refinement_ctx) {
            return Err(());
        }

        let insertion_ctx = config.initial_methods[method_idx].0.run(&mut refinement_ctx, empty_ctx.deep_copy());

        add_solution(&mut refinement_ctx, insertion_ctx);

        config.logger.deref()(format!(
            "[{}s] created {} of {} initial solutions in {}ms",
            evolution_time.elapsed_secs(),
            idx + 1,
            config.initial_size,
            item_time.elapsed_millis()
        ));

        Ok(())
    });

    Ok(refinement_ctx)
}

fn add_solution(refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) {
    let is_quota_reached = refinement_ctx.quota.as_ref().map_or(false, |quota| quota.is_reached());
    let is_population_empty = refinement_ctx.population.size() == 0;

    // NOTE fix population not to accept solution with worse primary objective fitness as best
    if is_population_empty || !is_quota_reached {
        refinement_ctx.population.add(insertion_ctx);
    }
}

fn log_progress(
    refinement_ctx: &RefinementContext,
    evolution_time: &Timer,
    generation_time: Option<&Timer>,
    logger: &Logger,
) {
    if let Some(best_individual) = refinement_ctx.population.best() {
        let best_fitness = refinement_ctx.problem.objective.fitness(best_individual);

        if refinement_ctx.generation % 100 == 0 {
            log_individual(
                best_individual,
                generation_time.map(|timer| (refinement_ctx.generation, timer)),
                (best_fitness, None),
                &evolution_time,
                logger,
            );
        }

        if refinement_ctx.generation % 1000 == 0 || refinement_ctx.generation == 1 {
            log_population(&refinement_ctx, &evolution_time, logger);
        }
    } else {
        logger.deref()("no progress yet".to_string());
    }
}

fn log_individual(
    insertion_ctx: &InsertionContext,
    generation: Option<(usize, &Timer)>,
    fitness: (f64, Option<f64>),
    evolution_time: &Timer,
    logger: &Logger,
) {
    let (fitness_value, fitness_change) = fitness;
    let fitness_values = insertion_ctx
        .problem
        .objective
        .objectives()
        .map(|objective| objective.fitness(insertion_ctx))
        .map(|fitness| format!("{:.3}", fitness))
        .collect::<Vec<_>>();

    logger.deref()(format!(
        "{}cost: {:.2}{}, tours: {}, unassigned: {}, fitness: ({})",
        generation.map_or("\t".to_string(), |(gen, time)| format!(
            "[{}s] generation {} took {}ms, ",
            evolution_time.elapsed_secs(),
            gen,
            time.elapsed_millis()
        )),
        fitness_value,
        fitness_change.map_or_else(|| "".to_string(), |change| format!(" ({:.3}%)", change)),
        insertion_ctx.solution.routes.len(),
        insertion_ctx.solution.unassigned.len(),
        fitness_values.join(", ")
    ));
}

fn log_population(refinement_ctx: &RefinementContext, evolution_time: &Timer, logger: &Logger) {
    logger.deref()(format!(
        "[{}s] population state (speed: {:.2} gen/sec):",
        evolution_time.elapsed_secs(),
        refinement_ctx.generation as f64 / evolution_time.elapsed_secs_as_f64(),
    ));

    refinement_ctx.population.all().for_each(|insertion_ctx| {
        log_individual(insertion_ctx, None, get_fitness(&refinement_ctx, &insertion_ctx), evolution_time, logger)
    });
}

fn log_result(refinement_ctx: &RefinementContext, evolution_time: &Timer, logger: &Logger) {
    log_population(refinement_ctx, evolution_time, logger);
    logger.deref()(format!(
        "[{}s] total generations: {}, speed: {:.2} gen/sec",
        evolution_time.elapsed_secs(),
        refinement_ctx.generation,
        refinement_ctx.generation as f64 / evolution_time.elapsed_secs_as_f64()
    ));
}

fn get_fitness(refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> (f64, Option<f64>) {
    let fitness_value = refinement_ctx.problem.objective.fitness(insertion_ctx);

    let fitness_change = refinement_ctx
        .population
        .best()
        .map(|best_ctx| refinement_ctx.problem.objective.fitness(best_ctx))
        .map(|best_fitness| (fitness_value - best_fitness) / best_fitness * 100.);

    (fitness_value, fitness_change)
}
