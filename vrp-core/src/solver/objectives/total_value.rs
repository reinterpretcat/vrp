#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_value_test.rs"]
mod total_value_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::{Job, TargetConstraint, TargetObjective};
use crate::solver::objectives::GenericValue;
use std::ops::Deref;
use std::sync::Arc;

/// A type which provides functionality needed to maximize total value of served jobs.
pub struct TotalValue {}

impl TotalValue {
    /// Creates _(constraint, objective)_  type pair which provides logic to maximize total value.
    pub fn maximize(
        max_value: f64,
        reduction_factor: f64,
        value_func: Arc<dyn Fn(&Job) -> f64 + Send + Sync>,
    ) -> (TargetConstraint, TargetObjective) {
        assert!(max_value > 0.);

        GenericValue::new_constrained_objective(
            None,
            None,
            Arc::new({
                let value_func = value_func.clone();
                move |rc: &RouteContext| rc.route.tour.jobs().map(|job| value_func.deref()(&job)).sum()
            }),
            Arc::new(move |ctx: &SolutionContext| {
                ctx.routes.iter().map(|rc| rc.state.get_route_state(TOTAL_VALUE_KEY).cloned().unwrap_or(0.)).sum()
            }),
            Arc::new(move |_, _, job, max_cost| {
                let job_value = value_func.deref()(job);

                if max_cost > 0. {
                    -(job_value / max_value) * max_cost * reduction_factor
                } else {
                    -job_value * reduction_factor
                }
            }),
            TOTAL_VALUE_KEY,
        )
    }
}
