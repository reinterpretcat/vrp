use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, RandomProbability};
use crate::refinement::recreate::{CompositeRecreate, Recreate};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::selection::{SelectRandom, Selection};
use crate::refinement::termination::*;
use crate::refinement::RefinementContext;
use crate::utils::DefaultRandom;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

/// A basic implementation of ruin and recreate metaheuristic.
pub struct Solver {
    pub recreate: Box<dyn Recreate>,
    pub ruin: Box<dyn Ruin>,
    pub selection: Box<dyn Selection>,
    pub acceptance: Box<dyn Acceptance>,
    pub termination: Box<dyn Termination>,
    pub settings: SolverSettings,
    pub logger: Box<dyn Fn(String) -> ()>,
}

/// A solver settings.
pub struct SolverSettings {
    pub minimize_routes: bool,
    pub population_size: usize,
    pub init_insertion_ctx: Option<(InsertionContext, ObjectiveCost)>,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self { minimize_routes: false, population_size: 1, init_insertion_ctx: None }
    }
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(CompositeRecreate::default()),
            Box::new(CompositeRuin::default()),
            Box::new(SelectRandom::default()),
            Box::new(RandomProbability::default()),
            Box::new(CompositeTermination::default()),
            SolverSettings::default(),
            Box::new(|msg| println!("{}", msg)),
        )
    }
}

impl Solver {
    pub fn new(
        recreate: Box<dyn Recreate>,
        ruin: Box<dyn Ruin>,
        selection: Box<dyn Selection>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        settings: SolverSettings,
        logger: Box<dyn Fn(String) -> ()>,
    ) -> Self {
        Self { recreate, ruin, selection, acceptance, termination, settings, logger }
    }

    pub fn solve(&mut self, problem: Arc<Problem>) -> Option<(Solution, ObjectiveCost, usize)> {
        let mut refinement_ctx = RefinementContext::new(problem.clone(), self.settings.minimize_routes, 5);
        let mut insertion_ctx = match &self.settings.init_insertion_ctx {
            Some((ctx, cost)) => {
                refinement_ctx.population.add((ctx.deep_copy(), cost.clone(), 1));
                ctx.deep_copy()
            }
            None => InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())),
        };

        let refinement_time = Instant::now();
        loop {
            let generation_time = Instant::now();

            insertion_ctx = self.ruin.run(&refinement_ctx, insertion_ctx);
            insertion_ctx = self.recreate.run(&refinement_ctx, insertion_ctx);

            let cost = problem.objective.estimate(&insertion_ctx);
            let is_accepted = self.acceptance.is_accepted(&refinement_ctx, (&insertion_ctx, cost.clone()));
            let is_terminated =
                self.termination.is_termination(&refinement_ctx, (&insertion_ctx, cost.clone(), is_accepted));

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_accepted {
                self.log_generation(
                    &refinement_ctx,
                    generation_time,
                    refinement_time,
                    (&insertion_ctx, &cost),
                    is_accepted,
                );
            }

            if refinement_ctx.generation > 0 && refinement_ctx.generation % 1000 == 0 {
                self.log_population(&refinement_ctx, refinement_time);
            }

            if is_accepted {
                refinement_ctx.population.add((insertion_ctx, cost, refinement_ctx.generation))
            }

            insertion_ctx = self.selection.select(&refinement_ctx);

            if is_terminated {
                break;
            }

            refinement_ctx.generation += 1;
        }

        self.log_speed(&refinement_ctx, refinement_time);
        self.get_result(refinement_ctx)
    }

    fn log_generation(
        &self,
        refinement_ctx: &RefinementContext,
        generation_time: Instant,
        refinement_time: Instant,
        solution: (&InsertionContext, &ObjectiveCost),
        is_accepted: bool,
    ) {
        let (insertion_ctx, cost) = solution;
        let (actual_change, total_change) = self.get_cost_change(refinement_ctx, &cost);
        self.logger.deref()(format!(
            "generation {} took {}ms (total {}s), cost: ({:.2},{:.2}): ({:.3}%, {:.3}%), routes: {}, accepted: {}",
            refinement_ctx.generation,
            generation_time.elapsed().as_millis(),
            refinement_time.elapsed().as_secs(),
            cost.actual,
            cost.penalty,
            actual_change,
            total_change,
            insertion_ctx.solution.routes.len(),
            is_accepted
        ));
    }

    fn log_population(&self, refinement_ctx: &RefinementContext, refinement_time: Instant) {
        self.logger.deref()(format!("\tpopulation state after {}s:", refinement_time.elapsed().as_secs()));
        refinement_ctx.population.all(self.settings.minimize_routes).enumerate().for_each(
            |(idx, (insertion_ctx, cost, generation))| {
                let (actual_change, total_change) = self.get_cost_change(refinement_ctx, cost);
                self.logger.deref()(format!(
                    "\t\t{} cost: ({:.2},{:.2}): ({:.3}%, {:.3}%), routes: {}, discovered at: {}",
                    idx,
                    cost.actual,
                    cost.penalty,
                    actual_change,
                    total_change,
                    insertion_ctx.solution.routes.len(),
                    generation
                ))
            },
        );
    }

    fn log_speed(&self, refinement_ctx: &RefinementContext, refinement_time: Instant) {
        let elapsed = refinement_time.elapsed();
        self.logger.deref()(format!(
            "Solving took {} ms, total generations: {}, speed: {:.2} generations/sec",
            elapsed.as_millis(),
            refinement_ctx.generation,
            refinement_ctx.generation as f64 / elapsed.as_secs_f64()
        ));
    }

    fn get_result(&self, refinement_ctx: RefinementContext) -> Option<(Solution, ObjectiveCost, usize)> {
        let best = refinement_ctx.population.best(self.settings.minimize_routes);
        if let Some((ctx, cost, generation)) = best {
            self.logger.deref()(format!(
                "Best solution within cost {} discovered at {} generation",
                cost.total(),
                generation
            ));
            Some((ctx.solution.to_solution(refinement_ctx.problem.extras.clone()), cost.clone(), *generation))
        } else {
            None
        }
    }

    fn get_cost_change(&self, refinement_ctx: &RefinementContext, cost: &ObjectiveCost) -> (f64, f64) {
        refinement_ctx
            .population
            .best(self.settings.minimize_routes)
            .map(|(_, c, _)| {
                ((cost.actual - c.actual) / c.actual * 100., (cost.total() - c.total()) / c.total() * 100.)
            })
            .unwrap_or((100., 100.))
    }
}
