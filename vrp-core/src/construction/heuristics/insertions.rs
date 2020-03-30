use crate::construction::heuristics::evaluators::{evaluate_job_insertion, InsertionPosition};
use crate::construction::states::{InsertionContext, InsertionResult, Quota};
use crate::models::problem::Job;
use crate::utils::map_reduce;
use std::ops::Deref;

/// On each insertion step, selects a list of jobs to be inserted.
/// It is up to implementation to decide whether a list is original consists of jobs to be inserted,
/// subset, randomized or something else.
pub trait JobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a>;
}

/// Returns a list of all jobs to be inserted.
pub struct AllJobSelector {}

impl Default for AllJobSelector {
    fn default() -> Self {
        Self {}
    }
}

impl JobSelector for AllJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        Box::new(ctx.solution.required.iter().cloned())
    }
}

/// Reduces job collection into single insertion result.
pub trait JobMapReducer {
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        map: Box<dyn Fn(&Job) -> InsertionResult + Send + Sync + 'a>,
    ) -> InsertionResult;
}

/// A job map reducer which compares pairs of insertion results and pick one from those.
pub struct PairJobMapReducer {
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl PairJobMapReducer {
    pub fn new(result_selector: Box<dyn ResultSelector + Send + Sync>) -> Self {
        Self { result_selector }
    }
}

impl JobMapReducer for PairJobMapReducer {
    fn reduce<'a>(
        &'a self,
        ctx: &'a InsertionContext,
        jobs: Vec<Job>,
        map: Box<dyn Fn(&Job) -> InsertionResult + Send + Sync + 'a>,
    ) -> InsertionResult {
        map_reduce(
            &jobs,
            |job| map.deref()(&job),
            InsertionResult::make_failure,
            |a, b| self.result_selector.select(&ctx, a, b),
        )
    }
}

/// Selects one insertion result from two to promote as best.
pub trait ResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult;
}

/// Selects best result.
pub struct BestResultSelector {}

impl Default for BestResultSelector {
    fn default() -> Self {
        Self {}
    }
}

impl ResultSelector for BestResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(left, right)
    }
}

/// Implements generalized insertion heuristic.
/// Using [`JobSelector`] and [`ResultSelector`], it tries to identify next job to be inserted until
/// there are no jobs left or it is not possible to insert due to constraint limitations.
pub struct InsertionHeuristic {
    insertion_position: InsertionPosition,
}

impl Default for InsertionHeuristic {
    fn default() -> Self {
        InsertionHeuristic::new(InsertionPosition::Any)
    }
}

impl InsertionHeuristic {
    pub fn new(insertion_position: InsertionPosition) -> Self {
        Self { insertion_position }
    }
}

impl InsertionHeuristic {
    pub fn process(
        &self,
        job_selector: &Box<dyn JobSelector + Send + Sync>,
        job_reducer: &Box<dyn JobMapReducer + Send + Sync>,
        ctx: InsertionContext,
        quota: Option<&Box<dyn Quota + Send + Sync>>,
    ) -> InsertionContext {
        let mut ctx = ctx;

        prepare_ctx(&mut ctx);

        while !ctx.solution.required.is_empty() && !quota.map_or(false, |q| q.is_reached()) {
            let jobs = job_selector.select(&mut ctx).collect::<Vec<Job>>();
            let result = job_reducer.reduce(
                &ctx,
                jobs,
                Box::new(|job| evaluate_job_insertion(&job, &ctx, self.insertion_position)),
            );
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
    ctx.solution.unassigned.extend(ctx.solution.required.drain(0..).map(|job| (job, 0)));
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

            ctx.solution.required.retain(|j| *j != job);
            ctx.problem.constraint.accept_insertion(&mut ctx.solution, &mut success.context, &job);
        }
        InsertionResult::Failure(failure) => {
            if let Some(job) = failure.job {
                ctx.solution.unassigned.insert(job.clone(), failure.constraint);
                ctx.solution.required.retain(|j| *j != job);
            }
        }
    }
}
