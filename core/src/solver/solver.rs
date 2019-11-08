use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::objectives::{Objective, PenalizeUnassigned};
use crate::refinement::recreate::{CompositeRecreate, Recreate};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::selection::{SelectBest, Selection};
use crate::refinement::termination::*;
use crate::refinement::RefinementContext;
use crate::utils::{compare_floats, DefaultRandom};
use std::cmp::Ordering::{Greater, Less};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

/// A basic implementation of ruin and recreate metaheuristic.
pub struct Solver {
    pub recreate: Box<dyn Recreate>,
    pub ruin: Box<dyn Ruin>,
    pub selection: Box<dyn Selection>,
    pub objective: Box<dyn Objective>,
    pub acceptance: Box<dyn Acceptance>,
    pub termination: Box<dyn Termination>,
    pub settings: SolverSettings,
    pub logger: Box<dyn Fn(String) -> ()>,
}

/// A solver settings.
pub struct SolverSettings {
    pub minimize_routes: bool,
    pub population_size: usize,
    pub init_insertion_ctx: Option<InsertionContext>,
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
            Box::new(SelectBest::default()),
            Box::new(PenalizeUnassigned::default()),
            Box::new(Greedy::default()),
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
        objective: Box<dyn Objective>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        settings: SolverSettings,
        logger: Box<dyn Fn(String) -> ()>,
    ) -> Self {
        Self { recreate, ruin, selection, objective, acceptance, termination, settings, logger }
    }

    pub fn solve(&mut self, problem: Arc<Problem>) -> Option<(Solution, ObjectiveCost, usize)> {
        let mut refinement_ctx = RefinementContext::new(problem.clone());
        let mut insertion_ctx = match &self.settings.init_insertion_ctx {
            Some(ctx) => ctx.deep_copy(),
            None => InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new())),
        };

        let refinement_time = Instant::now();
        loop {
            let generation_time = Instant::now();

            insertion_ctx = self.ruin.run(insertion_ctx);
            insertion_ctx = self.recreate.run(insertion_ctx);

            let cost = self.objective.estimate(&insertion_ctx);
            let is_accepted = self.acceptance.is_accepted(&refinement_ctx, (&insertion_ctx, cost.clone()));
            let is_terminated =
                self.termination.is_termination(&refinement_ctx, (&insertion_ctx, cost.clone(), is_accepted));

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_accepted {
                self.log_generation(&refinement_ctx, generation_time, (&insertion_ctx, &cost), is_accepted);
            }

            if is_accepted {
                self.add_solution(&mut refinement_ctx, (insertion_ctx, cost));
            }

            insertion_ctx = self.selection.select(&refinement_ctx);

            if is_terminated {
                break;
            }

            refinement_ctx.generation = refinement_ctx.generation + 1;
        }

        self.log_speed(&refinement_ctx, refinement_time);
        self.get_result(refinement_ctx)
    }

    fn add_solution(&self, refinement_ctx: &mut RefinementContext, solution: (InsertionContext, ObjectiveCost)) {
        refinement_ctx.population.push((solution.0, solution.1, refinement_ctx.generation));
        refinement_ctx.population.sort_by(|(a_ctx, a_cost, _), (b_ctx, b_cost, _)| {
            match (a_ctx.solution.routes.len().cmp(&b_ctx.solution.routes.len()), self.settings.minimize_routes) {
                (Less, true) => Less,
                (Greater, true) => Greater,
                _ => compare_floats(&a_cost.total(), &b_cost.total()),
            }
        });
        refinement_ctx.population.truncate(self.settings.population_size);
    }

    fn log_generation(
        &self,
        refinement_ctx: &RefinementContext,
        generation_time: Instant,
        solution: (&InsertionContext, &ObjectiveCost),
        is_accepted: bool,
    ) {
        let (insertion_ctx, cost) = solution;
        self.logger.deref()(format!(
            "generation {} took {}ms, cost: ({:.2},{:.2}): {:.3}%, routes: {}, accepted: {}",
            refinement_ctx.generation,
            generation_time.elapsed().as_millis(),
            cost.actual,
            cost.penalty,
            refinement_ctx
                .population
                .first()
                .and_then(|(_, c, _)| Some((cost.total() - c.total()) / c.total() * 100.))
                .unwrap_or(100.),
            insertion_ctx.solution.routes.len(),
            is_accepted
        ));
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
        if refinement_ctx.population.is_empty() {
            None
        } else {
            let mut refinement_ctx = refinement_ctx;
            let (ctx, cost, generation) = refinement_ctx.population.remove(0);
            self.logger.deref()(format!(
                "Best solution within cost {} discovered at {} generation",
                cost.total(),
                generation
            ));
            Some((ctx.solution.to_solution(refinement_ctx.problem.extras.clone()), cost, generation))
        }
    }
}
