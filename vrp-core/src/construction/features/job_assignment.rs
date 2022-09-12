//! Provides the way to control job assignment.

use super::*;
use crate::models::problem::{Actor, Single};
use crate::utils::Either;
use std::cmp::Ordering;
use std::ops::Deref;

/// A type which allows to control how job is estimated in objective fitness.
pub type UnassignedJobEstimator = Arc<dyn Fn(&SolutionContext, &Job) -> f64 + Send + Sync>;

/// Creates a feature to minimize amount of unassigned jobs.
pub fn minimize_unassigned_jobs(unassigned_job_estimator: UnassignedJobEstimator) -> Result<Feature, String> {
    FeatureBuilder::default().with_objective(MinimizeUnassignedObjective { unassigned_job_estimator }).build()
}

/// Specifies a job value function which takes into account actor and job.
pub type ActorValueFn = Arc<dyn Fn(&Actor, &Job) -> f64 + Send + Sync>;
/// Specifies an job value function which takes into account only job.
pub type SimpleValueFn = Arc<dyn Fn(&Job) -> f64 + Send + Sync>;
/// Specifies a job value reader as a variant of two functions.
pub type JobReadValueFn = Either<SimpleValueFn, ActorValueFn>;
/// Specifies a job write value.
pub type JobWriteValueFn = Arc<dyn Fn(Job, f64) -> Job + Send + Sync>;
/// A job value estimation function.
type EstimateValueFn = Arc<dyn Fn(&SolutionContext, &RouteContext, &Job) -> f64 + Send + Sync>;

/// Maximizes a total value of served jobs.
pub fn maximize_total_job_value(
    job_read_value_fn: JobReadValueFn,
    job_write_value_fn: JobWriteValueFn,
    merge_code: ViolationCode,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(MaximizeTotalValueObjective {
            estimate_value_fn: Arc::new({
                let job_read_value_fn = job_read_value_fn.clone();
                move |solution, route_ctx, job| match &job_read_value_fn {
                    JobReadValueFn::Left(left) => left.deref()(job),
                    JobReadValueFn::Right(right) => right.deref()(route_ctx.route.actor.as_ref(), job),
                }
            }),
        })
        .with_constraint(MaximizeTotalValueConstraint { merge_code, job_read_value_fn, job_write_value_fn })
        .build()
}

struct MinimizeUnassignedObjective {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl Objective for MinimizeUnassignedObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .unassigned
            .iter()
            .map(|(job, _)| self.unassigned_job_estimator.deref()(&solution.solution, job))
            .sum::<f64>()
    }
}

impl FeatureObjective for MinimizeUnassignedObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, job, .. } => {
                -1. * self.unassigned_job_estimator.deref()(solution_ctx, job)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct MaximizeTotalValueObjective {
    estimate_value_fn: EstimateValueFn,
}

impl Objective for MaximizeTotalValueObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        todo!()
    }
}

impl FeatureObjective for MaximizeTotalValueObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job, .. } => {
                self.estimate_value_fn.deref()(solution_ctx, route_ctx, job)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct MaximizeTotalValueConstraint {
    merge_code: ViolationCode,
    job_read_value_fn: JobReadValueFn,
    job_write_value_fn: JobWriteValueFn,
}

impl FeatureConstraint for MaximizeTotalValueConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        unimplemented!()
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match &self.job_read_value_fn {
            JobReadValueFn::Left(left) => {
                let source_value = left.deref()(&source);
                let candidate_value = left.deref()(&candidate);
                let new_value = source_value + candidate_value;

                Ok(if compare_floats(new_value, source_value) != Ordering::Equal {
                    self.job_write_value_fn.deref()(source, new_value)
                } else {
                    source
                })
            }
            JobReadValueFn::Right(_) => Err(self.merge_code),
        }
    }
}
