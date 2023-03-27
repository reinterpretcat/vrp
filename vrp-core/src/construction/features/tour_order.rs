//! Provides way to enforce job activity ordering in the tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_order_test.rs"]
mod tour_order_test;

use super::*;
use crate::models::problem::{Actor, Single};
use crate::models::solution::Activity;
use crate::utils::Either;
use std::cmp::Ordering;
use std::ops::Deref;

/// Creates a tour order feature as hard constraint.
pub fn create_tour_order_hard_feature(
    name: &str,
    code: ViolationCode,
    order_fn: TourOrderFn,
) -> Result<Feature, String> {
    FeatureBuilder::default().with_name(name).with_constraint(TourOrderConstraint { code, order_fn }).build()
}

/// Creates a tour order as soft constraint.
pub fn create_tour_order_soft_feature(
    name: &str,
    state_key: StateKey,
    order_fn: TourOrderFn,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TourOrderObjective { state_key, order_fn: order_fn.clone() })
        .with_state(TourOrderState { state_key, state_keys: vec![state_key], order_fn })
        .build()
}

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
pub type ActorTourOrderFn = Arc<dyn Fn(&Actor, Option<&Single>) -> OrderResult + Send + Sync>;

/// Specifies an activity order function which takes into account only single job.
pub type SingleTourOrderFn = Arc<dyn Fn(Option<&Single>) -> OrderResult + Send + Sync>;

/// Specifies an order func as a variant of two functions.
pub type TourOrderFn = Either<SingleTourOrderFn, ActorTourOrderFn>;

struct TourOrderConstraint {
    code: ViolationCode,
    order_fn: TourOrderFn,
}

impl FeatureConstraint for TourOrderConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Activity { route_ctx, activity_ctx } => {
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
            Either::Left(left) => {
                let order_fn = left.deref();
                let order_fn_cmp = |source: &Single, candidate: &Single| {
                    let source = order_fn(Some(source));
                    let candidate = order_fn(Some(candidate));
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
}

struct TourOrderObjective {
    state_key: StateKey,
    order_fn: TourOrderFn,
}

impl Objective for TourOrderObjective {
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

impl FeatureObjective for TourOrderObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Activity { route_ctx, activity_ctx } => {
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
    state_key: StateKey,
    state_keys: Vec<StateKey>,
    order_fn: TourOrderFn,
}

impl FeatureState for TourOrderState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let violations = get_violations(solution_ctx.routes.as_slice(), &self.order_fn);
        solution_ctx.state.insert(self.state_key, Arc::new(violations));
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

fn evaluate_result<T>(
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    order_fn: &TourOrderFn,
    check_order: &(dyn Fn(OrderResult, OrderResult, bool) -> Option<T>),
) -> Option<T> {
    let get_order = |single: Option<&Single>| match &order_fn {
        Either::Left(left) => left.deref()(single),
        Either::Right(right) => right.deref()(route_ctx.route().actor.as_ref(), single),
    };
    let get_order_by_idx = |idx: usize| get_order(route_ctx.route().tour.get(idx).and_then(get_single));
    let target = get_order(get_single(activity_ctx.target));

    unwrap_from_result(
        (0..=activity_ctx.index)
            .map(get_order_by_idx)
            .map(|early| (early, target, true))
            .chain(
                (activity_ctx.index + 1..route_ctx.route().tour.total())
                    .map(get_order_by_idx)
                    .map(|late| (target, late, false)),
            )
            .try_fold(None, |_, (left, right, stopped)| {
                let result = check_order.deref()(left, right, stopped);
                if result.is_some() {
                    Err(result)
                } else {
                    Ok(None)
                }
            }),
    )
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
                    Either::Left(left) => left.deref()(Some(single.as_ref())),
                    Either::Right(right) => right.deref()(route_ctx.route().actor.as_ref(), Some(single.as_ref())),
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
        (OrderResult::Value(left), OrderResult::Value(right)) => compare_floats(left, right),
        (OrderResult::Value(_), OrderResult::Default) => Ordering::Less,
        (OrderResult::Default, OrderResult::Value(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
