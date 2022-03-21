#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/tour_order_test.rs"]
mod tour_order_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::problem::*;
use crate::utils::Either;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// Specifies order result.
#[derive(Copy, Clone)]
pub enum OrderResult {
    /// Returns a specified value.
    Value(f64),
    /// No value specified.
    Default,
    /// No value specified, but constraint should be ignored
    Ignored,
}

/// Specifies an activity order function which takes into account actor and single job.
pub type ActorOrderFn = Arc<dyn Fn(&Actor, &Single) -> OrderResult + Send + Sync>;

/// Specifies an activity order function which takes into account only single job.
pub type SingleOrderFn = Arc<dyn Fn(&Single) -> OrderResult + Send + Sync>;

/// Specifies an order func as a variant of two functions.
pub type OrderFn = Either<SingleOrderFn, ActorOrderFn>;

/// Allows to control desired activity order in tours.
pub struct TourOrder {}

impl TourOrder {
    /// Creates instances of unconstrained tour order logic. Unconstrained means that a job with less
    /// order can be assigned after a job with larger order in the tour. Violations are counted by the
    /// objective.
    pub fn new_unconstrained(order_fn: OrderFn, state_key: i32) -> (TargetConstraint, TargetObjective) {
        Self::new_objective(order_fn, state_key, None)
    }

    /// Creates instances of constrained tour order logic: a job with less order cannot be assigned after
    /// a job with larger order in the tour.
    pub fn new_constrained(
        order_fn: OrderFn,
        state_key: i32,
        constraint_code: i32,
    ) -> (TargetConstraint, TargetObjective) {
        Self::new_objective(order_fn, state_key, Some(constraint_code))
    }

    fn new_objective(
        order_fn: OrderFn,
        state_key: i32,
        constraint_code: Option<i32>,
    ) -> (TargetConstraint, TargetObjective) {
        let constraints = if let Some(constraint_code) = constraint_code {
            vec![
                ConstraintVariant::SoftActivity(Arc::new(TourOrderSoftActivityConstraint {
                    order_fn: order_fn.clone(),
                })),
                ConstraintVariant::HardActivity(Arc::new(TourOrderHardActivityConstraint {
                    order_fn: order_fn.clone(),
                    constraint_code,
                })),
            ]
        } else {
            vec![ConstraintVariant::SoftActivity(Arc::new(TourOrderSoftActivityConstraint {
                order_fn: order_fn.clone(),
            }))]
        };

        let constraint = TourOrderConstraint {
            code: constraint_code.unwrap_or(-1),
            constraints,
            keys: vec![state_key],
            order_fn: order_fn.clone(),
        };

        // TODO do not use this objective for constrained variant as there should be no violations?
        let objective = OrderActivityObjective { order_fn, state_key };

        (Arc::new(constraint), Arc::new(objective))
    }
}

struct TourOrderConstraint {
    code: i32,
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
    order_fn: OrderFn,
}

impl ConstraintModule for TourOrderConstraint {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        if let Some(state_key) = self.keys.first() {
            let violations = get_violations(ctx.routes.as_slice(), &self.order_fn);
            ctx.state.insert(*state_key, Arc::new(violations));
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        match &self.order_fn {
            Either::Left(left) => {
                let order_fn = left.deref();
                let order_fn_cmp = |source: &Single, candidate: &Single| {
                    let source = order_fn(source);
                    let candidate = order_fn(candidate);
                    match (source, candidate) {
                        (OrderResult::Value(s), OrderResult::Value(c)) => compare_floats(s, c) == Ordering::Equal,
                        (OrderResult::Default, OrderResult::Default) | (OrderResult::Ignored, OrderResult::Ignored) => {
                            true
                        }
                        _ => false,
                    }
                };

                match (&source, &candidate) {
                    (Job::Single(s_source), Job::Single(s_candidate)) if order_fn_cmp(s_source, s_candidate) => {
                        Ok(source)
                    }
                    _ => Err(self.code),
                }
            }
            Either::Right(_) => Err(self.code),
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct TourOrderHardActivityConstraint {
    order_fn: OrderFn,
    constraint_code: i32,
}

impl HardActivityConstraint for TourOrderHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        evaluate_result(route_ctx, activity_ctx, &self.order_fn, &|first, second, stopped| {
            if compare_order_results(first, second) == Ordering::Greater {
                Some(ActivityConstraintViolation { code: self.constraint_code, stopped })
            } else {
                None
            }
        })
    }
}

struct TourOrderSoftActivityConstraint {
    order_fn: OrderFn,
}

impl SoftActivityConstraint for TourOrderSoftActivityConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> f64 {
        evaluate_result(route_ctx, activity_ctx, &self.order_fn, &|first, second, _| {
            if compare_order_results(first, second) == Ordering::Greater {
                let max_cost = route_ctx.get_route_cost();
                let penalty = if compare_floats(max_cost, 0.) == Ordering::Equal { 1E9 } else { max_cost * 2. };

                let value = match (first, second) {
                    (OrderResult::Value(first), OrderResult::Value(second)) => first - second,
                    (OrderResult::Default, OrderResult::Value(value)) => -value,
                    (OrderResult::Value(value), OrderResult::Default) => value,
                    _ => 0.,
                };

                Some(value * penalty)
            } else {
                None
            }
        })
        .unwrap_or(0.)
    }
}

struct OrderActivityObjective {
    order_fn: OrderFn,
    state_key: i32,
}

impl Objective for OrderActivityObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        let solution = &solution.solution;

        solution
            .state
            .get(&self.state_key)
            .and_then(|s| s.downcast_ref::<usize>())
            .cloned()
            .unwrap_or_else(|| get_violations(solution.routes.as_slice(), &self.order_fn)) as f64
    }
}

fn evaluate_result<T>(
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    order_fn: &OrderFn,
    check_order: &(dyn Fn(OrderResult, OrderResult, bool) -> Option<T>),
) -> Option<T> {
    let prev = activity_ctx.prev.job.as_ref();
    let target = activity_ctx.target.job.as_ref();
    let next = activity_ctx.next.and_then(|next| next.job.as_ref());

    let actor = route_ctx.route.actor.as_ref();

    let get_order = |single: &Single| match order_fn {
        Either::Left(left) => left.deref()(single),
        Either::Right(right) => right.deref()(actor, single),
    };

    match (prev, target, next) {
        (Some(prev), Some(target), None) => check_order.deref()(get_order(prev), get_order(target), true),
        (None, Some(target), Some(next)) => check_order.deref()(get_order(target), get_order(next), false),
        (Some(prev), Some(target), Some(next)) => check_order.deref()(get_order(prev), get_order(target), true)
            .or_else(|| check_order.deref()(get_order(target), get_order(next), false)),
        _ => None,
    }
}

fn get_violations(routes: &[RouteContext], order_fn: &OrderFn) -> usize {
    routes
        .iter()
        .map(|route_ctx| {
            let orders = route_ctx
                .route
                .tour
                .all_activities()
                .filter_map(|activity| activity.job.as_ref())
                .map(|single| match order_fn {
                    Either::Left(left) => left.deref()(single.as_ref()),
                    Either::Right(right) => right.deref()(route_ctx.route.actor.as_ref(), single.as_ref()),
                })
                .collect::<Vec<OrderResult>>();

            orders.windows(2).fold(0_usize, |acc, pair| {
                let value = match *pair {
                    [prev, next] => match compare_order_results(prev, next) {
                        Ordering::Greater => 1,
                        _ => 0,
                    },
                    _ => unreachable!(),
                };

                acc + value
            })
        })
        .sum::<usize>()
}

fn compare_order_results(left: OrderResult, right: OrderResult) -> Ordering {
    match (left, right) {
        (OrderResult::Value(left), OrderResult::Value(right)) => compare_floats(left, right),
        (OrderResult::Value(_), OrderResult::Default) => Ordering::Less,
        (OrderResult::Default, OrderResult::Value(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
