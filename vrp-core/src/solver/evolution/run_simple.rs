use crate::solver::evolution::*;
use crate::solver::{RefinementContext, Telemetry};
use crate::utils::Timer;

/// A simple evolution algorithm which maintains single population.
#[derive(Default)]
pub struct RunSimple {}

impl EvolutionStrategy for RunSimple {
    fn run(
        &self,
        refinement_ctx: RefinementContext,
        hyper: Box<dyn HyperHeuristic + Send + Sync>,
        termination: &(dyn Termination + Send + Sync),
        telemetry: Telemetry,
    ) -> EvolutionResult {
        let mut refinement_ctx = refinement_ctx;
        let mut hyper = hyper;
        let mut telemetry = telemetry;

        while !should_stop(&mut refinement_ctx, termination) {
            let generation_time = Timer::start();

            let parents = refinement_ctx.population.select().collect();

            let offspring = hyper.search(&refinement_ctx, parents);

            let is_improved =
                if should_add_solution(&refinement_ctx) { refinement_ctx.population.add_all(offspring) } else { false };

            on_generation(&mut refinement_ctx, &mut telemetry, termination, generation_time, is_improved);
        }

        telemetry.on_result(&refinement_ctx);

        Ok((refinement_ctx.population, telemetry.get_metrics()))
    }
}
