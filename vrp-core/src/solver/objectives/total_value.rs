#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_value_test.rs"]
mod total_value_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::{Job, TargetConstraint, TargetObjective};
use crate::solver::objectives::GenericValue;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

/// A type which provides functionality needed to maximize total value of served jobs.
pub struct TotalValue {}

impl TotalValue {
    /// Creates _(constraint, objective)_  type pair which provides logic to maximize total value.
    pub fn maximize(
        max_value: f64,
        reduction_factor: f64,
        solution_value_func: Arc<dyn Fn(&SolutionContext) -> f64 + Send + Sync>,
        job_read_value_func: Arc<dyn Fn(&Job) -> f64 + Send + Sync>,
        job_write_value_func: Arc<dyn Fn(Job, f64) -> Job + Send + Sync>,
    ) -> (TargetConstraint, TargetObjective) {
        assert!(max_value > 0.);

        let get_route_value = {
            let value_func = job_read_value_func.clone();
            Arc::new(move |rc: &RouteContext| rc.route.tour.jobs().map(|job| -value_func.deref()(&job)).sum())
        };

        GenericValue::new_constrained_objective(
            None,
            Arc::new({
                let job_read_value_func = job_read_value_func.clone();
                move |source, candidate| {
                    let source_value = job_read_value_func.deref()(&source);
                    let candidate_value = job_read_value_func.deref()(&candidate);
                    let new_value = source_value + candidate_value;

                    Ok(if compare_floats(new_value, source_value) != Ordering::Equal {
                        job_write_value_func.deref()(source, new_value)
                    } else {
                        source
                    })
                }
            }),
            get_route_value.clone(),
            Arc::new(move |ctx: &SolutionContext| {
                let route_values: f64 = ctx.routes.iter().map(|rc| get_route_value(rc)).sum();
                let solution_values: f64 = -solution_value_func.deref()(ctx);

                route_values + solution_values
            }),
            Arc::new(move |_, _, job, max_cost| {
                let job_value = -job_read_value_func.deref()(job);

                if max_cost > 0. {
                    (job_value / max_value) * max_cost * reduction_factor
                } else {
                    job_value * reduction_factor
                }
            }),
            TOTAL_VALUE_KEY,
        )
    }
}
