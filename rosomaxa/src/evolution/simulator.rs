use crate::evolution::{EvolutionResult, EvolutionStrategy};
use crate::prelude::*;
use crate::utils::Timer;
use std::marker::PhantomData;

/// An entity which simulates evolution process.
pub struct EvolutionSimulator<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    config: EvolutionConfig<C, O, S>,
}

impl<C, O, S> EvolutionSimulator<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `EvolutionSimulator`.
    pub fn new(config: EvolutionConfig<C, O, S>) -> Result<Self, String> {
        if config.initial.operators.is_empty() {
            return Err("at least one initial method has to be specified".to_string());
        }

        Ok(Self { config })
    }

    /// Runs evolution for given `problem` using evolution `config`.
    /// Returns populations filled with solutions.
    pub fn run(self) -> EvolutionResult<S> {
        let mut config = self.config;

        let hooks = config.processing;
        let random = config.context.environment().random.clone();

        let heuristic_ctx = config.context;
        let logger = heuristic_ctx.environment().logger.clone();

        let mut heuristic_ctx = hooks.context.iter().fold(heuristic_ctx, |ctx, hook| hook.pre_process(ctx));

        (logger)("preparing initial solution(-s)");
        std::mem::take(&mut config.initial.individuals).into_iter().take(config.initial.max_size).for_each(
            |solution| {
                heuristic_ctx.on_initial(solution, Timer::start());
            },
        );

        let weights = config.initial.operators.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();

        let init_size = heuristic_ctx.population().size();
        let init_time = Timer::start();
        let _ = (init_size..config.initial.max_size).try_for_each(|idx| {
            let item_time = Timer::start();

            let is_overall_termination = config.termination.is_termination(&mut heuristic_ctx);
            let is_initial_quota_reached = config.termination.estimate(&heuristic_ctx) > config.initial.quota;

            if is_initial_quota_reached || is_overall_termination {
                (logger)(
                    format!(
                        "stop building initial solutions due to initial quota reached ({is_initial_quota_reached}) or overall termination ({is_overall_termination}).",
                    )
                        .as_str(),
                );
                return Err(());
            }

            let operator_idx = if idx < config.initial.operators.len() {
                idx
            } else {
                random.weighted(weights.as_slice())
            };

            // TODO consider initial quota limit
            let solution = config.initial.operators[operator_idx].0.create(&heuristic_ctx);
            heuristic_ctx.on_initial(solution, item_time);

            Ok(())
        });

        if heuristic_ctx.population().size() > 0 {
            (logger)(&format!("created initial population in {}ms", init_time.elapsed_millis()));
        } else {
            (logger)("created an empty population");
        }

        config.strategy.as_ref().run(heuristic_ctx, config.heuristic, config.termination).map(|(solutions, metrics)| {
            let solutions = solutions
                .into_iter()
                .map(|solution| hooks.solution.iter().fold(solution, |s, hook| hook.post_process(s)))
                .collect();

            (solutions, metrics)
        })
    }
}

/// A simple evolution algorithm which maintains single population.
pub struct RunSimple<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    desired_solutions_amount: usize,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<C, O, S> RunSimple<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `RunSimple`.
    pub fn new(desired_solutions_amount: usize) -> Self {
        Self { desired_solutions_amount, _marker: (Default::default(), Default::default(), Default::default()) }
    }
}

impl<C, O, S> EvolutionStrategy for RunSimple<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn run(
        &self,
        heuristic_ctx: Self::Context,
        heuristic: Box<
            dyn HyperHeuristic<Context = Self::Context, Objective = Self::Objective, Solution = Self::Solution>,
        >,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution> {
        let mut heuristic_ctx = heuristic_ctx;
        let mut heuristic = heuristic;

        loop {
            let is_terminated = termination.is_termination(&mut heuristic_ctx);
            let is_quota_reached = heuristic_ctx.environment().quota.as_ref().map_or(false, |q| q.is_reached());

            if is_terminated || is_quota_reached {
                break;
            }

            let generation_time = Timer::start();

            let parents = heuristic_ctx.population().select().collect::<Vec<_>>();

            let diverse_offspring = if heuristic_ctx.population().selection_phase() == SelectionPhase::Exploitation {
                Vec::default()
            } else {
                heuristic.diversify(&heuristic_ctx, parents.clone())
            };

            let search_offspring = heuristic.search(&heuristic_ctx, parents);

            let offspring = search_offspring.into_iter().chain(diverse_offspring).collect::<Vec<_>>();

            let termination_estimate = termination.estimate(&heuristic_ctx);

            heuristic_ctx.on_generation(offspring, termination_estimate, generation_time);
        }

        // NOTE give a chance to report internal state of heuristic
        (heuristic_ctx.environment().logger)(&format!("{heuristic}"));

        let (population, telemetry_metrics) = heuristic_ctx.on_result()?;

        let solutions =
            population.ranked().map(|(solution, _)| solution.deep_copy()).take(self.desired_solutions_amount).collect();

        Ok((solutions, telemetry_metrics))
    }
}

impl<C, O, S> Default for RunSimple<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn default() -> Self {
        Self::new(1)
    }
}
