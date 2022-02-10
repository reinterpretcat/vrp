#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_value_test.rs"]
mod total_value_test;

use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::{Actor, Job, TargetConstraint, TargetObjective};
use crate::solver::objectives::{GenericValue, SolutionValueFn};
use crate::utils::Either;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

/// Specifies a job value function which takes into account actor and job.
pub type ActorValueFn = Arc<dyn Fn(&Actor, &Job) -> f64 + Send + Sync>;
/// Specifies an job value function which takes into account only job.
pub type SimpleValueFn = Arc<dyn Fn(&Job) -> f64 + Send + Sync>;
/// Specifies a value func as a variant of two functions.
pub type ValueFn = Either<SimpleValueFn, ActorValueFn>;
/// Specifies a job write value.
pub type JobWriteValueFn = Arc<dyn Fn(Job, f64) -> Job + Send + Sync>;

/// A type which provides functionality needed to maximize total value of served jobs.
pub struct TotalValue {}

impl TotalValue {
    /// Creates _(constraint, objective)_  type pair which provides logic to maximize total value.
    pub fn maximize(
        max_value: f64,
        reduction_factor: f64,
        solution_value_func: SolutionValueFn,
        job_read_value_func: ValueFn,
        job_write_value_func: JobWriteValueFn,
        state_key: i32,
        merge_code: i32,
    ) -> (TargetConstraint, TargetObjective) {
        assert!(max_value > 0.);

        let get_route_value = {
            let value_func = job_read_value_func.clone();
            Arc::new(move |rc: &RouteContext| {
                rc.route
                    .tour
                    .jobs()
                    .map(|job| {
                        -1. * match &value_func {
                            ValueFn::Left(left) => left.deref()(&job),
                            ValueFn::Right(right) => right.deref()(rc.route.actor.as_ref(), &job),
                        }
                    })
                    .sum()
            })
        };

        GenericValue::new_constrained_objective(
            None,
            Arc::new({
                let job_read_value_func = job_read_value_func.clone();
                move |source, candidate| match &job_read_value_func {
                    ValueFn::Left(left) => {
                        let source_value = left.deref()(&source);
                        let candidate_value = left.deref()(&candidate);
                        let new_value = source_value + candidate_value;

                        Ok(if compare_floats(new_value, source_value) != Ordering::Equal {
                            job_write_value_func.deref()(source, new_value)
                        } else {
                            source
                        })
                    }
                    ValueFn::Right(_) => Err(merge_code),
                }
            }),
            get_route_value.clone(),
            Arc::new(move |ctx: &SolutionContext| {
                let route_values: f64 = ctx.routes.iter().map(|rc| get_route_value(rc)).sum();
                let solution_values: f64 = -solution_value_func.deref()(ctx);

                route_values + solution_values
            }),
            Arc::new(move |_, route_ctx, job, max_cost| {
                let job_value = match &job_read_value_func {
                    ValueFn::Left(left) => left.deref()(job),
                    ValueFn::Right(right) => right.deref()(route_ctx.route.actor.as_ref(), job),
                } * -1.;

                if max_cost > 0. {
                    (job_value / max_value) * max_cost * reduction_factor
                } else {
                    job_value * reduction_factor
                }
            }),
            state_key,
        )
    }
}
