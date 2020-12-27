use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::Problem;
use crate::solver::evolution::*;
use crate::solver::population::{Greedy, Individual};
use crate::solver::TelemetryMode;
use crate::utils::{parallel_into_collect, Random};
use std::cmp::Ordering::Greater;
use std::sync::Arc;

/// An evolution strategy which runs first evolution over decomposed list of routes.
pub struct RunDecompose {
    inner_strategy: Arc<dyn EvolutionStrategy + Send + Sync>,
    generations: usize,
}

impl EvolutionStrategy for RunDecompose {
    fn run(
        &self,
        refinement_ctx: RefinementContext,
        mutation: &(dyn Mutation + Send + Sync),
        termination: &(dyn Termination + Send + Sync),
        telemetry: Telemetry,
    ) -> EvolutionResult {
        let mut refinement_ctx = refinement_ctx;

        let refinement_result = refinement_ctx
            .population
            .ranked()
            .next()
            .and_then(|(individual, _)| {
                decompose_individual(&refinement_ctx, individual).map(|result| (individual.random.clone(), result))
            })
            .map(|(random, decomposed_contexts)| {
                self.refine_decomposed(refinement_ctx.problem.clone(), random, mutation, decomposed_contexts)
            });

        if let Some(result) = refinement_result {
            let individual = result?;
            refinement_ctx.population.add(individual);
        }

        self.inner_strategy.run(refinement_ctx, mutation, termination, telemetry)
    }
}

impl RunDecompose {
    fn refine_decomposed(
        &self,
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        mutation: &(dyn Mutation + Send + Sync),
        decomposed_contexts: Vec<RefinementContext>,
    ) -> Result<Individual, String> {
        // do actual refinement independently for each decomposed context
        let evolution_results = parallel_into_collect(decomposed_contexts, |decomposed_ctx| {
            self.inner_strategy.run(
                decomposed_ctx,
                mutation,
                &MaxGeneration::new(self.generations),
                Telemetry::new(TelemetryMode::None),
            )
        });

        // merge evolution results into one individual
        let mut individual = evolution_results.into_iter().try_fold::<_, _, Result<_, String>>(
            Individual::new(problem.clone(), random),
            |mut individual, evolution_result| {
                let (population, _) = evolution_result?;
                let (decomposed_ctx, _) =
                    population.ranked().next().ok_or_else(|| "cannot get individual from population".to_string())?;

                let acc_solution = &mut individual.solution;
                let dec_solution = &decomposed_ctx.solution;

                // NOTE theoretically, we can avoid deep copy here, but this would require extension in Population trait
                acc_solution.routes.extend(dec_solution.routes.iter().map(|route_ctx| route_ctx.deep_copy()));
                acc_solution.ignored.extend(dec_solution.ignored.iter().cloned());
                acc_solution.required.extend(dec_solution.required.iter().cloned());
                acc_solution.locked.extend(dec_solution.locked.iter().cloned());
                acc_solution.unassigned.extend(dec_solution.unassigned.iter().map(|(k, v)| (k.clone(), v.clone())));

                Ok(individual)
            },
        )?;

        problem.constraint.accept_solution_state(&mut individual.solution);

        Ok(individual)
    }
}

fn create_population(individual: Individual) -> Box<dyn Population + Send + Sync> {
    Box::new(Greedy::new(individual.problem.clone(), Some(individual)))
}

fn create_route_groups(individual: &Individual) -> Vec<Vec<RouteContext>> {
    unimplemented!()
}

fn create_solution(individual: &Individual, routes: Vec<RouteContext>) -> SolutionContext {
    // TODO ensure locks/ignored jobs are propagated properly
    unimplemented!()
}

fn decompose_individual(refinement_ctx: &RefinementContext, individual: &Individual) -> Option<Vec<RefinementContext>> {
    let contexts = create_route_groups(individual)
        .into_iter()
        .map(|routes| create_solution(individual, routes))
        .map(|solution| Individual { problem: individual.problem.clone(), solution, random: individual.random.clone() })
        .map(|individual| RefinementContext {
            problem: refinement_ctx.problem.clone(),
            population: create_population(individual),
            state: Default::default(),
            quota: refinement_ctx.quota.clone(),
            statistics: Default::default(),
        })
        .collect::<Vec<_>>();

    if contexts.len() > 1 {
        Some(contexts)
    } else {
        None
    }
}
