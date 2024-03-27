use crate::evolution::EvolutionResult;
use crate::prelude::*;
use crate::utils::Timer;

/// An entity which simulates evolution process.
pub struct EvolutionSimulator<C, O, S, R>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    R: Random,
{
    config: EvolutionConfig<C, O, S, R>,
}

impl<C, O, S, R> EvolutionSimulator<C, O, S, R>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    R: Random,
{
    /// Creates a new instance of `EvolutionSimulator`.
    pub fn new(config: EvolutionConfig<C, O, S, R>) -> Result<Self, GenericError> {
        if config.initial.operators.is_empty() {
            return Err("at least one initial method has to be specified".into());
        }

        Ok(Self { config })
    }

    /// Runs evolution for given `problem` using evolution `config`.
    /// Returns populations filled with solutions.
    pub fn run(self) -> EvolutionResult<S> {
        let mut config = self.config;

        let hooks = config.processing;
        let environment = config.environment.clone();
        let random = environment.random.clone();
        let logger = environment.logger.clone();

        let heuristic_ctx = config.context;

        let mut heuristic_ctx = hooks.context.iter().fold(heuristic_ctx, |ctx, hook| hook.pre_process(ctx));

        (logger)("preparing initial solution(-s)");
        let init_size = std::mem::take(&mut config.initial.individuals).into_iter().take(config.initial.max_size).fold(
            0,
            |acc, solution| {
                heuristic_ctx.on_initial(solution, Timer::start());
                acc + 1
            },
        );

        let weights = config.initial.operators.iter().map(|(_, weight)| *weight).collect::<Vec<_>>();
        let init_time = Timer::start();
        let _ = (init_size..config.initial.max_size).try_for_each(|idx| {
            let item_time = Timer::start();

            let is_overall_termination = config.termination.is_termination(&mut heuristic_ctx);
            let is_initial_quota_reached = config.termination.estimate(&heuristic_ctx) > config.initial.quota;

            if is_initial_quota_reached || is_overall_termination {
                (logger)(
                    format!(
                        "stop building initial solutions due to initial quota reached ({is_initial_quota_reached})\
                         or overall termination ({is_overall_termination}).",
                    )
                    .as_str(),
                );
                return Err(());
            }

            let operator_idx =
                if idx < config.initial.operators.len() { idx } else { random.weighted(weights.as_slice()) };

            // TODO consider initial quota limit
            let solution = config.initial.operators[operator_idx].0.create(&heuristic_ctx);
            heuristic_ctx.on_initial(solution, item_time);

            Ok(())
        });

        (logger)(&format!("created initial population in {}ms", init_time.elapsed_millis()));

        config.strategy.run(heuristic_ctx, config.termination).map(|(solutions, metrics)| {
            let solutions = solutions
                .into_iter()
                .map(|solution| hooks.solution.iter().fold(solution, |s, hook| hook.post_process(s)))
                .collect();

            (solutions, metrics)
        })
    }
}
