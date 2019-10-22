use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::objectives::{Objective, PenalizeUnassigned};
use crate::refinement::recreate::{Recreate, CompositeRecreate};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::selection::{SelectBest, Selection};
use crate::refinement::termination::{MaxGeneration, Termination};
use crate::refinement::RefinementContext;
use crate::utils::DefaultRandom;
use std::cmp::Ordering::Less;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Provides configurable way to build solver.
pub struct SolverBuilder {
    solver: Solver,
    population_size: Option<usize>,
    minimize_routes: Option<bool>,
    max_generations: Option<usize>,
}

impl SolverBuilder {
    pub fn new() -> Self {
        Self { solver: Solver::default(), population_size: None, minimize_routes: None, max_generations: None }
    }

    pub fn with_population_size(&mut self, limit: usize) -> &mut Self {
        self.population_size = Some(limit);
        self
    }

    pub fn with_minimize_routes(&mut self, value: bool) -> &mut Self {
        self.minimize_routes = Some(value);
        self
    }

    pub fn with_max_generations(&mut self, limit: usize) -> &mut Self {
        self.max_generations = Some(limit);
        self
    }

    pub fn build(&mut self) -> Solver {
        // TODO support more parameters

        if let Some(limit) = self.max_generations {
            self.solver.logger.deref()(format!("configured to use generation limit: {}", limit).as_str());
            self.solver.termination = Box::new(MaxGeneration::new(limit));
        }

        if let Some(limit) = self.population_size {
            self.solver.logger.deref()(format!("configured to use population size: {}", limit).as_str());
            self.solver.population_size = limit;
        }

        if let Some(value) = self.minimize_routes {
            self.solver.logger.deref()(format!("configured to use minimize routes: {}", value).as_str());
            self.solver.acceptance = Box::new(Greedy::new(value));
        }

        std::mem::replace(&mut self.solver, Solver::default())
    }
}

/// A custom implementation of ruin and recreate metaheuristic.
pub struct Solver {
    recreate: Box<dyn Recreate>,
    ruin: Box<dyn Ruin>,
    selection: Box<dyn Selection>,
    objective: Box<dyn Objective>,
    acceptance: Box<dyn Acceptance>,
    termination: Box<dyn Termination>,
    logger: Box<dyn Fn(&str) -> ()>,
    population_size: usize,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(CompositeRecreate::default()),
            Box::new(CompositeRuin::default()),
            Box::new(SelectBest::default()),
            Box::new(PenalizeUnassigned::default()),
            Box::new(Greedy::default()),
            Box::new(MaxGeneration::default()),
            1,
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
        population_size: usize,
        logger: Box<dyn Fn(&str) -> ()>,
    ) -> Self {
        Self { recreate, ruin, selection, objective, acceptance, termination, population_size, logger }
    }

    pub fn solve(&self, problem: Problem) -> Option<(Solution, ObjectiveCost, usize)> {
        let problem = Arc::new(problem);
        let mut refinement_ctx = RefinementContext::new(problem.clone());
        let mut insertion_ctx = InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new()));

        let refinement_time = Instant::now();
        loop {
            let generation_time = Instant::now();

            insertion_ctx = self.ruin.run(insertion_ctx);
            insertion_ctx = self.recreate.run(insertion_ctx);

            let cost = self.objective.estimate(&insertion_ctx);
            let is_accepted = self.acceptance.is_accepted(&refinement_ctx, (&insertion_ctx, cost.clone()));
            let is_terminated =
                self.termination.is_termination(&refinement_ctx, (&insertion_ctx, cost.clone(), is_accepted));
            let routes = insertion_ctx.solution.routes.len();

            if is_accepted {
                refinement_ctx.population.push((insertion_ctx, cost.clone(), refinement_ctx.generation));
                refinement_ctx
                    .population
                    .sort_by(|(_, a, _), (_, b, _)| a.total().partial_cmp(&b.total()).unwrap_or(Less));
                refinement_ctx.population.truncate(self.population_size);
            }

            insertion_ctx = self.selection.select(&refinement_ctx);

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_accepted {
                self.logger.deref()(
                    format!(
                        "generation {} took {}ms, cost: ({:.2},{:.2}) routes: {}, accepted: {}",
                        refinement_ctx.generation,
                        generation_time.elapsed().as_millis(),
                        cost.actual,
                        cost.penalty,
                        routes,
                        is_accepted
                    )
                    .as_str(),
                );
            }

            if is_terminated {
                break;
            }

            refinement_ctx.generation = refinement_ctx.generation + 1;
        }

        self.log_speed(&refinement_ctx, refinement_time);
        self.get_result(refinement_ctx)
    }

    fn log_speed(&self, refinement_ctx: &RefinementContext, refinement_time: Instant) {
        let elapsed = refinement_time.elapsed();
        self.logger.deref()(
            format!(
                "Solving took {} ms, total generations: {}, speed: {:.2} generations/sec",
                elapsed.as_millis(),
                refinement_ctx.generation,
                refinement_ctx.generation as f64 / elapsed.as_secs_f64()
            )
            .as_str(),
        );
    }

    fn get_result(&self, refinement_ctx: RefinementContext) -> Option<(Solution, ObjectiveCost, usize)> {
        if refinement_ctx.population.is_empty() {
            None
        } else {
            let mut refinement_ctx = refinement_ctx;
            let (ctx, cost, generation) = refinement_ctx.population.remove(0);
            self.logger.deref()(
                format!("Best solution within cost {} discovered at {} generation", cost.total(), generation).as_str(),
            );
            Some((ctx.solution.to_solution(refinement_ctx.problem.extras.clone()), cost, generation))
        }
    }
}
