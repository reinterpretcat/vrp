use crate::constraints::{AREA_ORDER_KEY, AREA_VALUE_KEY};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::*;
use vrp_core::models::problem::Job;
use vrp_core::models::problem::*;
use vrp_core::solver::objectives::*;

/// An area module provides way to restrict given actor to work in specific areas.
pub struct AreaModule {
    keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    modules: Vec<TargetConstraint>,
}

impl AreaModule {
    /// Creates instances of unconstrained area logic. Unconstrained means that a job from area with
    /// less order can be assigned after a job from area with a larger order in the tour.
    /// Violations are counted by the objective.
    pub fn new_unconstrained(
        order_fn: ActorOrderFn,
        value_fn: ActorValueFn,
        solution_fn: SolutionValueFn,
        max_value: f64,
    ) -> (TargetConstraint, Vec<TargetObjective>) {
        Self::new_objective(order_fn, value_fn, solution_fn, max_value, None)
    }

    /// Creates instances of constrained area logic. Constrained means that a job from area with
    /// less order cannot be assigned after a job from area with a larger order in the tour.
    pub fn new_constrained(
        order_fn: ActorOrderFn,
        value_fn: ActorValueFn,
        solution_fn: SolutionValueFn,
        max_value: f64,
        constraint_code: i32,
    ) -> (TargetConstraint, Vec<TargetObjective>) {
        Self::new_objective(order_fn, value_fn, solution_fn, max_value, Some(constraint_code))
    }

    fn new_objective(
        order_fn: ActorOrderFn,
        value_fn: ActorValueFn,
        solution_fn: SolutionValueFn,
        max_value: f64,
        constraint_code: Option<i32>,
    ) -> (TargetConstraint, Vec<TargetObjective>) {
        let order_fn = OrderFn::Right(order_fn);

        let (order_constraint, order_objective) = if let Some(constraint_code) = constraint_code {
            TourOrder::new_constrained(order_fn, AREA_ORDER_KEY, constraint_code)
        } else {
            TourOrder::new_unconstrained(order_fn, AREA_ORDER_KEY)
        };

        let (value_constraint, value_objective) = TotalValue::maximize(
            max_value,
            0.1,
            solution_fn,
            ValueFn::Right(value_fn),
            Arc::new(|_, _| unreachable!()),
            AREA_VALUE_KEY,
            -1,
        );

        let area_module = Self {
            keys: vec![AREA_ORDER_KEY, AREA_VALUE_KEY],
            constraints: order_constraint
                .get_constraints()
                .chain(value_constraint.get_constraints())
                .cloned()
                .collect(),
            modules: vec![order_constraint, value_constraint],
        };

        (Arc::new(area_module), vec![order_objective, value_objective])
    }
}

impl ConstraintModule for AreaModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.modules.iter().for_each(|module| module.accept_insertion(solution_ctx, route_index, job));
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.modules.iter().for_each(|module| module.accept_route_state(ctx));
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        self.modules.iter().for_each(|module| module.accept_solution_state(ctx));
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}
