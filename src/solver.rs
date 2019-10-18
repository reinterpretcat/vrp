use crate::construction::states::InsertionContext;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::objectives::{Objective, PenalizeUnassigned};
use crate::refinement::recreate::{Recreate, RecreateWithCheapest};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::termination::{MaxGeneration, Termination};
use crate::refinement::RefinementContext;
use crate::utils::DefaultRandom;
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

    pub fn solve(&self, problem: Problem) -> Solution {
        let problem = Arc::new(problem);
        let mut refinement_ctx = RefinementContext::new(problem.clone());
        let mut insertion_ctx = InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new()));

        loop {
            insertion_ctx = self.recreate.run(insertion_ctx);

            // let cost = self.objective.estimate(&insertion_ctx);
            //let insertion_ctx = self.acceptance.is_accepted(refinement_ctx, ())

            if true {
                break;
            }
        }

        insertion_ctx.solution.into_solution(problem.extras.clone())
    }

    //    fn run_measure_log<T>(&self, func: impl FnOnce() -> T, msg: &str) -> T {
    //        let now = Instant::now();
    //        let result = func();
    //        let elapsed = now.elapsed();
    //
    //        self.logger.deref()(format!("{} took {}s", msg, elapsed.as_secs()).as_str());
    //
    //        result
    //    }
}
