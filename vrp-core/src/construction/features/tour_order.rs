//! Provides way to enforce job activity ordering in the tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_order_test.rs"]
mod tour_order_test;

use super::*;
use crate::models::problem::{Actor, Single};
use crate::models::solution::Activity;
use crate::utils::Either;
use std::cmp::Ordering;
use std::ops::ControlFlow;

custom_solution_state!(TourOrderViolations typeof usize);

/// Creates a tour order feature as hard constraint.
pub fn create_tour_order_hard_feature(
    name: &str,
    code: ViolationCode,
    order_fn: TourOrderFn,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default().with_name(name).with_constraint(TourOrderConstraint { code, order_fn }).build()
}

/// Creates a tour order as soft constraint.
pub fn create_tour_order_soft_feature(name: &str, order_fn: TourOrderFn) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TourOrderObjective { order_fn: order_fn.clone() })
        .with_state(TourOrderState { order_fn })
        .build()
}

/// Specifies order result.
#[derive(Copy, Clone)]
pub enum OrderResult {
    /// Returns a specified value.
    Value(Float),
    /// No value specified.
    Default,
    /// No value specified, but constraint should be ignored
    Ignored,
}

/// Specifies an activity order function which takes into an account actor and single job.
pub type ActorTourOrderFn = Arc<dyn Fn(&Actor, &Single) -> OrderResult + Send + Sync>;

/// Specifies an activity order function which takes into account only a single job.
pub type SingleTourOrderFn = Arc<dyn Fn(&Single) -> OrderResult + Send + Sync>;

/// Specifies an order func as a variant of two functions.
pub type TourOrderFn = Either<SingleTourOrderFn, ActorTourOrderFn>;

struct TourOrderConstraint {
    code: ViolationCode,
    order_fn: TourOrderFn,
}

impl FeatureConstraint for TourOrderConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Activity { route_ctx, activity_ctx, .. } => {
                evaluate_result(route_ctx, activity_ctx, &self.order_fn, &|first, second, stopped| {
                    if compare_order_results(first, second) == Ordering::Greater {
                        Some(ConstraintViolation { code: self.code, stopped })
                    } else {
                        None
                    }
                })
            }
            MoveContext::Route { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match &self.order_fn {
            Either::Left(order_fn) => {
                let order_fn_cmp = |source: &Single, candidate: &Single| {
                    let source = (order_fn)(source);
                    let candidate = (order_fn)(candidate);
                    match (source, candidate) {
                        (OrderResult::Value(s), OrderResult::Value(c)) => s == c,
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
}

struct TourOrderObjective {
    order_fn: TourOrderFn,
}

impl FeatureObjective for TourOrderObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        let solution = &solution.solution;

        solution
            .state
            .get_tour_order_violations()
            .copied()
            .unwrap_or_else(|| get_violations(solution.routes.as_slice(), &self.order_fn)) as Float
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Activity { route_ctx, activity_ctx, .. } => {
                evaluate_result(route_ctx, activity_ctx, &self.order_fn, &|first, second, _| {
                    if compare_order_results(first, second) == Ordering::Greater {
                        let value = match (first, second) {
                            (OrderResult::Value(first), OrderResult::Value(second)) => first - second,
                            (OrderResult::Default, OrderResult::Value(value)) => -value,
                            (OrderResult::Value(value), OrderResult::Default) => value,
                            _ => Cost::default(),
                        };

                        Some(value)
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
            }
            MoveContext::Route { .. } => Cost::default(),
        }
    }
}

struct TourOrderState {
    order_fn: TourOrderFn,
}

impl FeatureState for TourOrderState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let violations = get_violations(solution_ctx.routes.as_slice(), &self.order_fn);
        solution_ctx.state.set_tour_order_violations(violations);
    }
}

fn evaluate_result<T>(
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    order_fn: &TourOrderFn,
    check_order: &dyn Fn(OrderResult, OrderResult, bool) -> Option<T>,
) -> Option<T> {
    let get_order = |single: &Single| match &order_fn {
        Either::Left(left) => (left)(single),
        Either::Right(right) => (right)(route_ctx.route().actor.as_ref(), single),
    };

    let target_order = activity_ctx.target.job.as_ref().map(|single| get_order(single)).unwrap_or(OrderResult::Ignored);

    let get_order_by_idx = |idx: usize| {
        route_ctx.route().tour.get(idx).and_then(get_single).map(get_order).unwrap_or(OrderResult::Ignored)
    };

    (0..=activity_ctx.index)
        .map(get_order_by_idx)
        .map(|early| (early, target_order, true))
        .chain(
            (activity_ctx.index + 1..route_ctx.route().tour.total())
                .map(get_order_by_idx)
                .map(|late| (target_order, late, false)),
        )
        .try_fold(None, |_, (left, right, stopped)| {
            let result = (check_order)(left, right, stopped);
            if result.is_some() { ControlFlow::Break(result) } else { ControlFlow::Continue(None) }
        })
        .unwrap_value()
}

fn get_violations(routes: &[RouteContext], order_fn: &TourOrderFn) -> usize {
    routes
        .iter()
        .map(|route_ctx| {
            let orders = route_ctx
                .route()
                .tour
                .all_activities()
                .filter_map(|activity| activity.job.as_ref())
                .map(|single| match order_fn {
                    Either::Left(left) => (left)(single.as_ref()),
                    Either::Right(right) => (right)(route_ctx.route().actor.as_ref(), single.as_ref()),
                })
                .filter(|order| !matches!(order, OrderResult::Ignored))
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

fn get_single(activity: &Activity) -> Option<&Single> {
    activity.job.as_ref().map(|single| single.as_ref())
}

fn compare_order_results(left: OrderResult, right: OrderResult) -> Ordering {
    match (left, right) {
        (OrderResult::Value(left), OrderResult::Value(right)) => left.total_cmp(&right),
        (OrderResult::Value(_), OrderResult::Default) => Ordering::Less,
        (OrderResult::Default, OrderResult::Value(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
