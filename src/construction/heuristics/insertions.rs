#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/insertions_test.rs"]
mod insertions_test;

extern crate rayon;

use self::rayon::slice::Iter;
use crate::construction::heuristics::evaluators::evaluate_job_insertion;
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::utils::compare_shared;
use rayon::prelude::*;
use std::sync::Arc;

/// Selects jobs to be inserted.
pub trait JobSelector {
    fn select<'a>(&'a self, ctx: &'a InsertionContext) -> Iter<Arc<Job>>;
}

/// Selects insertion result to be promoted from two.
pub trait ResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult;
}

/// Implements abstract insertion heuristic.
pub struct InsertionHeuristic {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl InsertionHeuristic {
    pub fn new(
        job_selector: Box<dyn JobSelector + Send + Sync>,
        result_selector: Box<dyn ResultSelector + Send + Sync>,
    ) -> Self {
        Self { job_selector, result_selector }
    }

    pub fn process(&self, ctx: InsertionContext) -> InsertionContext {
        let mut ctx = ctx;
        ctx.problem.constraint.accept_solution_state(&mut ctx.solution);

        while !ctx.solution.required.is_empty() {
            let result = self
                .job_selector
                .select(&ctx)
                .map(|job| evaluate_job_insertion(&job, &ctx))
                .reduce(|| InsertionResult::make_failure(), |a, b| self.result_selector.select(&ctx, a, b));

            Self::insert(result, &mut ctx);
        }

        ctx
    }

    fn insert(result: InsertionResult, ctx: &mut InsertionContext) {
        match result {
            InsertionResult::Success(mut success) => {
                let job = success.job;
                {
                    let route = success.context.route.read().unwrap();
                    ctx.solution.registry.use_actor(&route.actor);
                    if !ctx.solution.routes.contains(&success.context) {
                        ctx.solution.routes.push(success.context.clone());
                    }
                }
                {
                    let mut route = success.context.route.write().unwrap();
                    success.activities.into_iter().for_each(|(a, index)| {
                        route.tour.insert_at(a, index + 1);
                    });
                }

                ctx.solution.required.retain(|j| !compare_shared(j, &job));
                ctx.problem.constraint.accept_route_state(&mut success.context);
            }
            InsertionResult::Failure(failure) => {
                let mut unassigned = &mut ctx.solution.unassigned;
                ctx.solution.required.drain(..).for_each(|j| {
                    unassigned.insert(j.clone(), failure.constraint);
                });
            }
        }
        // TODO update progress
        ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
    }
}

/// Returns a list of all jobs to be inserted.
pub struct AllJobSelector {}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a InsertionContext) -> Iter<Arc<Job>> {
        ctx.solution.required.par_iter()
    }
}

/// Selects best result.
pub struct BestResultSelector {}

impl ResultSelector for BestResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

pub fn create_cheapest_insertion_heuristic() -> InsertionHeuristic {
    InsertionHeuristic::new(Box::new(AllJobSelector {}), Box::new(BestResultSelector {}))
}
