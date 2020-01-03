extern crate rayon;

use self::rayon::prelude::*;
use crate::construction::heuristics::evaluators::evaluate_job_insertion;
use crate::construction::states::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::utils::compare_shared;
use std::sync::Arc;

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether a list is original consists of jobs to be inserted,
/// subset, randomized or something else.
pub trait JobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a>;
}

/// Selects one insertion result from two to promote as best.
pub trait ResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult;
}

/// Implements generalized insertion heuristic.
/// Using [`JobSelector`] and [`ResultSelector`], it tries to identify next job to be inserted until
/// there are no jobs left or it is not possible to insert due to constraint limitations.
pub struct InsertionHeuristic {}

impl InsertionHeuristic {
    pub fn process(
        job_selector: &Box<dyn JobSelector + Send + Sync>,
        result_selector: &Box<dyn ResultSelector + Send + Sync>,
        ctx: InsertionContext,
    ) -> InsertionContext {
        let mut ctx = ctx;

        prepare_ctx(&mut ctx);

        while !ctx.solution.required.is_empty() {
            let jobs = job_selector.select(&mut ctx).collect::<Vec<Arc<Job>>>();
            let result = jobs
                .par_iter()
                .map(|job| evaluate_job_insertion(&job, &ctx))
                .reduce(InsertionResult::make_failure, |a, b| result_selector.select(&ctx, a, b));

            insert(result, &mut ctx);
        }

        finalize_ctx(&mut ctx);

        ctx
    }
}

fn prepare_ctx(ctx: &mut InsertionContext) {
    ctx.solution.required.extend(ctx.solution.unassigned.drain().map(|(job, _)| job));
    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

fn finalize_ctx(ctx: &mut InsertionContext) {
    ctx.problem.constraint.accept_solution_state(&mut ctx.solution);
}

fn insert(result: InsertionResult, ctx: &mut InsertionContext) {
    match result {
        InsertionResult::Success(mut success) => {
            let job = success.job;

            ctx.solution.registry.use_actor(&success.context.route.actor);
            if !ctx.solution.routes.contains(&success.context) {
                ctx.solution.routes.push(success.context.clone());
            }

            let route = success.context.route_mut();
            success.activities.into_iter().for_each(|(a, index)| {
                route.tour.insert_at(a, index + 1);
            });

            ctx.solution.required.retain(|j| !compare_shared(j, &job));
            ctx.problem.constraint.accept_insertion(&mut ctx.solution, &mut success.context, &job);
        }
        InsertionResult::Failure(failure) => {
            let unassigned = &mut ctx.solution.unassigned;
            ctx.solution.required.drain(..).for_each(|j| {
                unassigned.insert(j.clone(), failure.constraint);
            });
        }
    }
}
