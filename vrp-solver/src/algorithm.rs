use crate::population::DiversePopulation;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;
use vrp_core::construction::states::InsertionContext;
use vrp_core::models::common::ObjectiveCost;
use vrp_core::models::{Problem, Solution};
use vrp_core::refinement::acceptance::{Acceptance, RandomProbability};
use vrp_core::refinement::mutation::{Mutation, RuinAndRecreateMutation};
use vrp_core::refinement::selection::{SelectRandom, Selection};
use vrp_core::refinement::termination::*;
use vrp_core::refinement::{Individuum, RefinementContext};
use vrp_core::utils::DefaultRandom;

/// A skeleton of metaheuristic with default ruin and recreate implementation.
pub struct Solver {
    pub selection: Box<dyn Selection>,
    pub mutation: Box<dyn Mutation>,
    pub acceptance: Box<dyn Acceptance>,
    pub termination: Box<dyn Termination>,
    pub settings: SolverSettings,
    pub logger: Box<dyn Fn(String) -> ()>,
}

/// A solver settings.
pub struct SolverSettings {
    /// A flag which is used to check whether route minimization should be preferred over cost.
    /// Default is false.
    pub minimize_routes: bool,
    /// An initial solution within cost.
    pub init_insertion_ctx: Option<(InsertionContext, ObjectiveCost)>,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self { minimize_routes: false, init_insertion_ctx: None }
    }
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(SelectRandom::default()),
            Box::new(RuinAndRecreateMutation::default()),
            Box::new(RandomProbability::default()),
            Box::new(CompositeTermination::default()),
            SolverSettings::default(),
            Box::new(|msg| println!("{}", msg)),
        )
    }
}

impl Solver {
    /// Creates a new instance of [`Solver`].
    pub fn new(
        selection: Box<dyn Selection>,
        mutation: Box<dyn Mutation>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        settings: SolverSettings,
        logger: Box<dyn Fn(String) -> ()>,
    ) -> Self {
        Self { selection, mutation, acceptance, termination, settings, logger }
    }

    /// Solves given problem and returns solution, its cost and generation when it is found.
    /// Return None if no solution found.
    pub fn solve(&mut self, problem: Arc<Problem>) -> Option<(Solution, ObjectiveCost, usize)> {
        let mut refinement_ctx = RefinementContext::new_with_population(
            problem.clone(),
            Box::new(DiversePopulation::new(self.settings.minimize_routes, 5)),
        );
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

            insertion_ctx = self.mutation.mutate(&mut refinement_ctx, insertion_ctx);

            let cost = problem.objective.estimate(&mut refinement_ctx, &insertion_ctx);
            let individuum = (insertion_ctx, cost, refinement_ctx.generation);
            let is_accepted = self.acceptance.is_accepted(&mut refinement_ctx, &individuum);
            let is_terminated = self.termination.is_termination(&mut refinement_ctx, (&individuum, is_accepted));

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_accepted {
                self.log_generation(&refinement_ctx, generation_time, refinement_time, &individuum, is_accepted);
            }

            if refinement_ctx.generation > 0 && refinement_ctx.generation % 1000 == 0 {
                self.log_population(&refinement_ctx, refinement_time);
            }

            if is_accepted {
                refinement_ctx.population.add(individuum)
            }

            insertion_ctx = self.selection.select(&mut refinement_ctx);

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
        solution: &Individuum,
        is_accepted: bool,
    ) {
        let (insertion_ctx, cost, _) = solution;
        let (actual_change, total_change) = get_cost_change(refinement_ctx, &cost);
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
        self.logger.deref()(format!(
            "\tpopulation state after {}s (speed: {:.2} gen/sec):",
            refinement_time.elapsed().as_secs(),
            refinement_ctx.generation as f64 / refinement_time.elapsed().as_secs_f64(),
        ));
        refinement_ctx.population.all().enumerate().for_each(|(idx, (insertion_ctx, cost, generation))| {
            let (actual_change, total_change) = get_cost_change(refinement_ctx, cost);
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
        });
    }

    fn log_speed(&self, refinement_ctx: &RefinementContext, refinement_time: Instant) {
        let elapsed = refinement_time.elapsed();
        self.logger.deref()(format!(
            "Solving took {} ms, total generations: {}, speed: {:.2} gen/sec",
            elapsed.as_millis(),
            refinement_ctx.generation,
            refinement_ctx.generation as f64 / elapsed.as_secs_f64()
        ));
    }

    fn get_result(&self, refinement_ctx: RefinementContext) -> Option<(Solution, ObjectiveCost, usize)> {
        if let Some((ctx, cost, generation)) = refinement_ctx.population.best() {
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
}

fn get_cost_change(refinement_ctx: &RefinementContext, cost: &ObjectiveCost) -> (f64, f64) {
    refinement_ctx
        .population
        .best()
        .map(|(_, c, _)| ((cost.actual - c.actual) / c.actual * 100., (cost.total() - c.total()) / c.total() * 100.))
        .unwrap_or((100., 100.))
}
