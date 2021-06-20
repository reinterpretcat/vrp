#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/tour_order_test.rs"]
mod tour_order_test;

use crate::algorithms::nsga2::Objective;
use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::problem::*;
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::cmp::Ordering::Greater;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// Allows to control desired activity order in tours.
pub struct TourOrder {}

impl TourOrder {
    /// Creates instances of unconstrained tour order logic. Unconstrained means that more prioritized
    /// job can be assigned after less prioritized in the tour if it leads to a better solution.
    pub fn new_unconstrained(
        order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
    ) -> (TargetConstraint, TargetObjective) {
        let state_key = TOUR_ORDER_KEY;

        let constraint = TourOrderConstraint {
            constraints: vec![ConstraintVariant::SoftActivity(Arc::new(TourOrderSoftActivityConstraint {
                order_func: order_func.clone(),
            }))],
            keys: vec![state_key],
            order_func: order_func.clone(),
        };

        let objective = OrderActivityObjective { order_func, state_key };

        (Box::new(constraint), Box::new(objective))
    }

    /// Creates instances of constrained tour order logic: more prioritized jobs are not allowed to
    /// be assigned after less prioritized in the tour.
    pub fn new_constrained(
        order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
        constraint_code: i32,
    ) -> TargetConstraint {
        Box::new(TourOrderConstraint {
            constraints: vec![
                ConstraintVariant::SoftActivity(Arc::new(TourOrderSoftActivityConstraint {
                    order_func: order_func.clone(),
                })),
                ConstraintVariant::HardActivity(Arc::new(TourOrderHardActivityConstraint {
                    order_func: order_func.clone(),
                    constraint_code,
                })),
            ],
            keys: vec![],
            order_func,
        })
    }
}

struct TourOrderConstraint {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
    order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
}

impl ConstraintModule for TourOrderConstraint {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        if let Some(state_key) = self.keys.first() {
            let violations = get_violations(ctx.routes.as_slice(), self.order_func.as_ref());
            ctx.state.insert(*state_key, Arc::new(violations));
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
    order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
    constraint_code: i32,
}

impl HardActivityConstraint for TourOrderHardActivityConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        evaluate_result(activity_ctx, self.order_func.as_ref(), &|first, second, stopped| {
            if compare_floats(first, second) == Greater {
                Some(ActivityConstraintViolation { code: self.constraint_code, stopped })
            } else {
                None
            }
        })
    }
}

struct TourOrderSoftActivityConstraint {
    order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
}

impl SoftActivityConstraint for TourOrderSoftActivityConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> f64 {
        evaluate_result(activity_ctx, self.order_func.as_ref(), &|first, second, _| {
            if compare_floats(first, second) == Greater {
                let max_cost = route_ctx.get_route_cost();
                let penalty = if compare_floats(max_cost, 0.) == Ordering::Equal { 1E9 } else { max_cost * 2. };

                Some((first - second) * penalty)
            } else {
                None
            }
        })
        .unwrap_or(0.)
    }
}

struct OrderActivityObjective {
    order_func: Arc<dyn Fn(&Single) -> Option<f64> + Send + Sync>,
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
            .unwrap_or_else(|| get_violations(solution.routes.as_slice(), self.order_func.as_ref())) as f64
    }
}

fn evaluate_result<T>(
    activity_ctx: &ActivityContext,
    order_func: &(dyn Fn(&Single) -> Option<f64> + Send + Sync),
    check_order: &(dyn Fn(f64, f64, bool) -> Option<T>),
) -> Option<T> {
    let prev = activity_ctx.prev.job.as_ref();
    let target = activity_ctx.target.job.as_ref();
    let next = activity_ctx.next.and_then(|next| next.job.as_ref());

    let get_order = |single: &Single| order_func.deref()(single).unwrap_or(f64::MAX);

    match (prev, target, next) {
        (Some(prev), Some(target), None) => check_order.deref()(get_order(prev), get_order(target), true),
        (None, Some(target), Some(next)) => check_order.deref()(get_order(target), get_order(next), false),
        (Some(prev), Some(target), Some(next)) => check_order.deref()(get_order(prev), get_order(target), true)
            .or_else(|| check_order.deref()(get_order(target), get_order(next), false)),
        _ => None,
    }
}

fn get_violations(routes: &[RouteContext], order_func: &(dyn Fn(&Single) -> Option<f64>)) -> usize {
    routes
        .iter()
        .map(|route_ctx| {
            let priorities = route_ctx
                .route
                .tour
                .all_activities()
                .filter_map(|activity| activity.job.as_ref())
                .map(|single| order_func(single.as_ref()).unwrap_or(f64::MAX))
                .collect::<Vec<f64>>();

            priorities.windows(2).fold(0_usize, |acc, pair| {
                let value = match pair {
                    &[prev, next] => {
                        if prev > next {
                            1
                        } else {
                            0
                        }
                    }
                    _ => unreachable!(),
                };

                acc + value
            })
        })
        .sum::<usize>()
}
