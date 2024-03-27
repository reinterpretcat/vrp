use super::*;
use crate::utils::Timer;

/// A simple evolution algorithm which maintains a single population and improves it iteratively.
pub struct Iterative<C, O, S, R>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    R: Random,
{
    environment: Environment<R>,
    heuristic: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
    desired_solutions_amount: usize,
}

impl<C, O, S, R> Iterative<C, O, S, R>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    R: Random,
{
    /// Creates a new instance of `RunSimple`.
    pub fn new(
        environment: Environment<R>,
        heuristic: Box<dyn HyperHeuristic<Context = C, Objective = O, Solution = S>>,
        desired_solutions_amount: usize,
    ) -> Self {
        Self { environment, heuristic, desired_solutions_amount }
    }
}

impl<C, O, S, R> EvolutionStrategy for Iterative<C, O, S, R>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
    R: Random,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn run(
        &mut self,
        heuristic_ctx: Self::Context,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution> {
        let mut heuristic_ctx = heuristic_ctx;
        let heuristic = &mut self.heuristic;

        loop {
            let is_terminated = termination.is_termination(&mut heuristic_ctx);
            let is_quota_reached = self.environment.quota.as_ref().map_or(false, |q| q.is_reached());

            if is_terminated || is_quota_reached {
                break;
            }

            let generation_time = Timer::start();

            let parents = heuristic_ctx.selected().collect::<Vec<_>>();

            let diverse_offspring = if heuristic_ctx.selection_phase() == SelectionPhase::Exploitation {
                Vec::default()
            } else {
                heuristic.diversify_many(&heuristic_ctx, parents.clone())
            };

            let search_offspring = heuristic.search_many(&heuristic_ctx, parents);

            let offspring = search_offspring.into_iter().chain(diverse_offspring).collect::<Vec<_>>();

            let termination_estimate = termination.estimate(&heuristic_ctx);

            heuristic_ctx.on_generation(offspring, termination_estimate, generation_time);
        }

        // NOTE give a chance to report internal state of heuristic
        (self.environment.logger)(&format!("{heuristic}"));

        let (population, telemetry_metrics) = heuristic_ctx.on_result()?;

        let solutions =
            population.ranked().map(|(solution, _)| solution.deep_copy()).take(self.desired_solutions_amount).collect();

        Ok((solutions, telemetry_metrics))
    }
}
