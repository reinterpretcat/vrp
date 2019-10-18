use crate::construction::states::InsertionContext;
use crate::models::{Problem, Solution};
use crate::refinement::acceptance::{Acceptance, Greedy};
use crate::refinement::recreate::{Recreate, RecreateWithCheapest};
use crate::refinement::ruin::{CompositeRuin, Ruin};
use crate::refinement::termination::{MaxGeneration, Termination};
use crate::utils::DefaultRandom;
use std::ops::Deref;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Solver {
    recreate: Box<dyn Recreate>,
    ruin: Box<dyn Ruin>,
    acceptance: Box<dyn Acceptance>,
    termination: Box<dyn Termination>,
    logger: Box<dyn Fn(&str) -> ()>,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(RecreateWithCheapest::default()),
            Box::new(CompositeRuin::default()),
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
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        logger: Box<dyn Fn(&str) -> ()>,
    ) -> Self {
        Self { recreate, ruin, acceptance, termination, logger }
    }

    pub fn solve(&self, problem: Problem) -> Solution {
        let insertion_ctx = InsertionContext::new(Arc::new(problem), Arc::new(DefaultRandom::new()));

        let insertion_ctx = self.run_measure_log(|| self.recreate.run(insertion_ctx), "create initial solution");

        // TODO refine solution

        insertion_ctx.solution.into_solution(insertion_ctx.problem.extras.clone())
    }

    fn run_measure_log<T>(&self, func: impl FnOnce() -> T, msg: &str) -> T {
        let now = Instant::now();
        let result = func();
        let elapsed = now.elapsed();

        self.logger.deref()(format!("{} took {}s", msg, elapsed.as_secs()).as_str());

        result
    }
}
