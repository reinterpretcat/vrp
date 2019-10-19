use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::objectives::{Objective, PenalizeUnassigned};
use crate::refinement::recreate::{Recreate, RecreateWithCheapest};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::termination::{MaxGeneration, Termination};
use crate::refinement::RefinementContext;
use crate::utils::DefaultRandom;
use std::cmp::Ordering::Less;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Solver {
    recreate: Box<dyn Recreate>,
    ruin: Box<dyn Ruin>,
    objective: Box<dyn Objective>,
    acceptance: Box<dyn Acceptance>,
    termination: Box<dyn Termination>,
    logger: Box<dyn Fn(&str) -> ()>,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(RecreateWithCheapest::default()),
            Box::new(CompositeRuin::default()),
            Box::new(PenalizeUnassigned::default()),
            Box::new(Greedy::default()),
            Box::new(MaxGeneration::default()),
            Box::new(|msg| println!("{}", msg)),
        )
    }
}

impl Solver {
    pub fn new(
        recreate: Box<dyn Recreate>,
        ruin: Box<dyn Ruin>,
        objective: Box<dyn Objective>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        logger: Box<dyn Fn(&str) -> ()>,
    ) -> Self {
        Self { recreate, ruin, objective, acceptance, termination, logger }
    }

    pub fn solve(&self, problem: Problem) -> Option<(Solution, ObjectiveCost, usize)> {
        let problem = Arc::new(problem);
        let mut refinement_ctx = RefinementContext::new(problem.clone());
        let mut insertion_ctx = InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new()));

        loop {
            let now = Instant::now();

            insertion_ctx = self.ruin.run(insertion_ctx);
            insertion_ctx = self.recreate.run(insertion_ctx);

            let cost = self.objective.estimate(&insertion_ctx);
            let is_accepted = self.acceptance.is_accepted(&refinement_ctx, (&insertion_ctx, cost.clone()));
            let is_terminated =
                self.termination.is_termination(&refinement_ctx, (&insertion_ctx, cost.clone(), is_accepted));

            if is_accepted {
                refinement_ctx.population.push((
                    insertion_ctx.solution.to_solution(problem.extras.clone()),
                    cost.clone(),
                    refinement_ctx.generation,
                ));
                refinement_ctx
                    .population
                    .sort_by(|(_, a, _), (_, b, _)| a.total().partial_cmp(&b.total()).unwrap_or(Less))
            }

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_accepted {
                self.logger.deref()(
                    format!(
                        "iteration {} took {}ms, cost: ({},{}), accepted: {}",
                        refinement_ctx.generation,
                        now.elapsed().as_millis(),
                        cost.actual,
                        cost.penalty,
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

        if refinement_ctx.population.is_empty() {
            None
        } else {
            let solution = refinement_ctx.population.remove(0);
            self.logger.deref()(
                format!("Best solution within cost {} discovered at {} generation", solution.1.total(), solution.2)
                    .as_str(),
            );
            Some(solution)
        }
    }
}
