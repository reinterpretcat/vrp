#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/evaluators_test.rs"]
mod evaluators_test;

use std::sync::Arc;

use crate::construction::constraints::{ActivityConstraintViolation, ConstraintPipeline};
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Leg, Place};
use crate::utils::Either;
use rosomaxa::utils::unwrap_from_result;
use std::iter::repeat;

/// Specifies an evaluation context data.
pub struct EvaluationContext<'a> {
    /// An actual constraint.
    pub constraint: &'a ConstraintPipeline,
    /// A job which is about to be inserted.
    pub job: &'a Job,
    /// A leg selector.
    pub leg_selector: &'a (dyn LegSelector + Send + Sync),
    /// A result selector.
    pub result_selector: &'a (dyn ResultSelector + Send + Sync),
}

/// Specifies allowed insertion position in route for the job.
#[derive(Copy, Clone)]
pub enum InsertionPosition {
    /// Job can be inserted anywhere in the route.
    Any,
    /// Job can be inserted only at the leg with the concrete index.
    Concrete(usize),
    /// Job can be inserted only to the end of the route.
    Last,
}

/// Evaluates possibility to preform insertion from given insertion context in given route
/// at given position constraint.
pub fn evaluate_job_insertion_in_route(
    insertion_ctx: &InsertionContext,
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    alternative: InsertionResult,
) -> InsertionResult {
    // NOTE do not evaluate unassigned job in unmodified route if it has a concrete code
    match (route_ctx.is_stale(), insertion_ctx.solution.unassigned.get(eval_ctx.job)) {
        (false, Some(UnassignedCode::Simple(_))) | (false, Some(UnassignedCode::Detailed(_))) => return alternative,
        _ => {}
    }

    let constraint = &insertion_ctx.problem.constraint;

    if let Some(violation) = constraint.evaluate_hard_route(&insertion_ctx.solution, route_ctx, eval_ctx.job) {
        return eval_ctx.result_selector.select_insertion(
            insertion_ctx,
            alternative,
            InsertionResult::make_failure_with_code(violation.code, true, Some(eval_ctx.job.clone())),
        );
    }

    let route_costs = constraint.evaluate_soft_route(&insertion_ctx.solution, route_ctx, eval_ctx.job);
    let best_known_cost = match &alternative {
        InsertionResult::Success(success) => Some(success.cost),
        _ => None,
    };

    if let Some(best_known_cost) = best_known_cost {
        if best_known_cost < route_costs {
            return alternative;
        }
    }

    eval_ctx.result_selector.select_insertion(
        insertion_ctx,
        alternative,
        evaluate_job_constraint_in_route(eval_ctx, route_ctx, position, route_costs, best_known_cost),
    )
}

/// Evaluates possibility to preform insertion in route context only.
/// NOTE: doesn't evaluate constraints on route level.
pub fn evaluate_job_constraint_in_route(
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    match eval_ctx.job {
        Job::Single(single) => evaluate_single(eval_ctx, route_ctx, single, position, route_costs, best_known_cost),
        Job::Multi(multi) => evaluate_multi(eval_ctx, route_ctx, multi, position, route_costs, best_known_cost),
    }
}

pub(crate) fn evaluate_single_constraint_in_route(
    insertion_ctx: &InsertionContext,
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    single: &Arc<Single>,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    if let Some(violation) = eval_ctx.constraint.evaluate_hard_route(&insertion_ctx.solution, route_ctx, eval_ctx.job) {
        InsertionResult::Failure(InsertionFailure {
            constraint: violation.code,
            stopped: true,
            job: Some(eval_ctx.job.clone()),
        })
    } else {
        evaluate_single(eval_ctx, route_ctx, single, position, route_costs, best_known_cost)
    }
}

fn evaluate_single(
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    single: &Arc<Single>,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position);
    let mut activity = Activity::new_with_job(single.clone());

    let result = analyze_insertion_in_route(
        eval_ctx,
        route_ctx,
        insertion_idx,
        single,
        &mut activity,
        SingleContext::new(best_known_cost, 0),
    );

    let job = eval_ctx.job.clone();
    if result.is_success() {
        activity.place = result.place.unwrap();
        let activities = vec![(activity, result.index)];
        InsertionResult::make_success(result.cost.unwrap() + route_costs, job, activities, route_ctx.clone())
    } else {
        let (code, stopped) = result.violation.map_or((0, false), |v| (v.code, v.stopped));
        InsertionResult::make_failure_with_code(code, stopped, Some(job))
    }
}

fn evaluate_multi(
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    multi: &Arc<Multi>,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position).unwrap_or(0);
    // 1. analyze permutations
    let result = unwrap_from_result(multi.permutations().into_iter().try_fold(
        MultiContext::new(best_known_cost, insertion_idx),
        |acc_res, services| {
            let mut shadow = ShadowContext::new(eval_ctx.constraint, route_ctx);
            let perm_res = unwrap_from_result(repeat(0).try_fold(MultiContext::new(None, insertion_idx), |out, _| {
                if out.is_failure(route_ctx.route.tour.job_activity_count()) {
                    return Result::Err(out);
                }
                shadow.restore(route_ctx);

                // 2. analyze inner jobs
                let sq_res = unwrap_from_result(services.iter().try_fold(out.next(), |in1, service| {
                    if in1.violation.is_some() {
                        return Result::Err(in1);
                    }
                    let mut activity = Activity::new_with_job(service.clone());
                    // 3. analyze legs
                    let srv_res = analyze_insertion_in_route(
                        eval_ctx,
                        &shadow.ctx,
                        None,
                        service,
                        &mut activity,
                        SingleContext::new(None, in1.next_index),
                    );

                    if srv_res.is_success() {
                        activity.place = srv_res.place.unwrap();
                        let activity = shadow.insert(activity, srv_res.index);
                        let activities = concat_activities(in1.activities, (activity, srv_res.index));
                        return MultiContext::success(in1.cost.unwrap_or(0.) + srv_res.cost.unwrap(), activities);
                    }

                    MultiContext::fail(srv_res, in1)
                }));

                MultiContext::promote(sq_res, out)
            }));

            MultiContext::promote(perm_res, acc_res)
        },
    ));

    let job = eval_ctx.job.clone();
    if result.is_success() {
        let activities = result.activities.unwrap();
        InsertionResult::make_success(result.cost.unwrap() + route_costs, job, activities, route_ctx.clone())
    } else {
        let (code, stopped) = result.violation.map_or((0, false), |v| (v.code, v.stopped));
        InsertionResult::make_failure_with_code(code, stopped, Some(job))
    }
}

fn analyze_insertion_in_route(
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    insertion_idx: Option<usize>,
    single: &Single,
    target: &mut Activity,
    init: SingleContext,
) -> SingleContext {
    unwrap_from_result(match insertion_idx {
        Some(idx) => {
            if let Some(leg) = route_ctx.route.tour.legs().nth(idx) {
                analyze_insertion_in_route_leg(eval_ctx, route_ctx, leg, single, target, init)
            } else {
                Ok(init)
            }
        }
        None => eval_ctx
            .leg_selector
            .get_legs(route_ctx, eval_ctx.job, init.index)
            .try_fold(init, |out, leg| analyze_insertion_in_route_leg(eval_ctx, route_ctx, leg, single, target, out)),
    })
}

fn analyze_insertion_in_route_leg(
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    leg: Leg,
    single: &Single,
    target: &mut Activity,
    out: SingleContext,
) -> Result<SingleContext, SingleContext> {
    let (items, index) = leg;
    let (prev, next) = match items {
        [prev] => (prev, None),
        [prev, next] => (prev, Some(next)),
        _ => panic!("Unexpected route leg configuration."),
    };
    let start_time = route_ctx.route.tour.start().unwrap().schedule.departure;
    // analyze service details
    single.places.iter().try_fold(out, |in1, detail| {
        // analyze detail time windows
        detail.times.iter().try_fold(in1, |in2, time| {
            target.place = Place {
                location: detail.location.unwrap_or(prev.place.location),
                duration: detail.duration,
                time: time.to_time_window(start_time),
            };

            let activity_ctx = ActivityContext { index, prev, target, next };

            if let Some(violation) = eval_ctx.constraint.evaluate_hard_activity(route_ctx, &activity_ctx) {
                return SingleContext::fail(violation, in2);
            }

            let costs = eval_ctx.constraint.evaluate_soft_activity(route_ctx, &activity_ctx);
            let other_costs = in2.cost.unwrap_or(f64::MAX);

            match eval_ctx.result_selector.select_cost(route_ctx, costs, other_costs) {
                Either::Left(_) => SingleContext::success(activity_ctx.index, costs, target.place.clone()),
                Either::Right(_) => SingleContext::skip(in2),
            }
        })
    })
}

fn get_insertion_index(route_ctx: &RouteContext, position: InsertionPosition) -> Option<usize> {
    match position {
        InsertionPosition::Any => None,
        InsertionPosition::Concrete(idx) => Some(idx),
        InsertionPosition::Last => Some(route_ctx.route.tour.legs().count().max(1) - 1),
    }
}

/// Stores information needed for single insertion.
#[derive(Debug)]
struct SingleContext {
    /// Constraint violation.
    pub violation: Option<ActivityConstraintViolation>,
    /// Insertion index.
    pub index: usize,
    /// Best cost.
    pub cost: Option<Cost>,
    /// Activity place.
    pub place: Option<Place>,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Option<Cost>, index: usize) -> Self {
        Self { violation: None, index, cost, place: None }
    }

    fn fail(violation: ActivityConstraintViolation, other: SingleContext) -> Result<Self, Self> {
        let stopped = violation.stopped;
        let ctx = Self { violation: Some(violation), index: other.index, cost: other.cost, place: other.place };
        if stopped {
            Result::Err(ctx)
        } else {
            Result::Ok(ctx)
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn success(index: usize, cost: Cost, place: Place) -> Result<Self, Self> {
        Result::Ok(Self { violation: None, index, cost: Some(cost), place: Some(place) })
    }

    #[allow(clippy::unnecessary_wraps)]
    fn skip(other: SingleContext) -> Result<Self, Self> {
        Result::Ok(other)
    }

    fn is_success(&self) -> bool {
        self.place.is_some()
    }
}

/// Stores information needed for multi job insertion.
struct MultiContext {
    /// Constraint violation.
    pub violation: Option<ActivityConstraintViolation>,
    /// Insertion index for first service.
    pub start_index: usize,
    /// Insertion index for next service.
    pub next_index: usize,
    /// Cost accumulator.
    pub cost: Option<Cost>,
    /// Activities with their indices.
    pub activities: Option<Vec<(Activity, usize)>>,
}

impl MultiContext {
    /// Creates new empty insertion context.
    fn new(cost: Option<Cost>, index: usize) -> Self {
        Self { violation: None, start_index: index, next_index: index, cost, activities: None }
    }

    /// Promotes insertion context by best price.
    fn promote(left: Self, right: Self) -> Result<Self, Self> {
        let index = left.start_index.max(right.start_index) + 1;
        let best = match (left.cost, right.cost) {
            (Some(left_cost), Some(right_cost)) => {
                if left_cost < right_cost {
                    left
                } else {
                    right
                }
            }
            (Some(_), None) => left,
            (None, Some(_)) => right,
            _ => {
                if left.violation.is_some() {
                    left
                } else {
                    right
                }
            }
        };

        let result = Self {
            violation: best.violation,
            start_index: index,
            next_index: index,
            cost: best.cost,
            activities: best.activities,
        };

        if result.violation.as_ref().map_or_else(|| false, |v| v.stopped) {
            Result::Err(result)
        } else {
            Result::Ok(result)
        }
    }

    /// Creates failed insertion context within reason code.
    fn fail(err_ctx: SingleContext, other_ctx: MultiContext) -> Result<Self, Self> {
        let (code, stopped) =
            err_ctx.violation.map_or((0, false), |v| (v.code, v.stopped && other_ctx.activities.is_none()));

        Result::Err(Self {
            violation: Some(ActivityConstraintViolation { code, stopped }),
            start_index: other_ctx.start_index,
            next_index: other_ctx.start_index,
            cost: None,
            activities: None,
        })
    }

    /// Creates successful insertion context.
    #[allow(clippy::unnecessary_wraps)]
    fn success(cost: Cost, activities: Vec<(Activity, usize)>) -> Result<Self, Self> {
        Result::Ok(Self {
            violation: None,
            start_index: activities.first().unwrap().1,
            next_index: activities.last().unwrap().1 + 1,
            cost: Some(cost),
            activities: Some(activities),
        })
    }

    /// Creates next insertion context from existing one.
    fn next(&self) -> Self {
        Self {
            violation: None,
            start_index: self.start_index,
            next_index: self.start_index,
            cost: None,
            activities: None,
        }
    }

    /// Checks whether insertion is found.
    fn is_success(&self) -> bool {
        self.violation.is_none() && self.cost.is_some() && self.activities.is_some()
    }

    /// Checks whether insertion is failed.
    fn is_failure(&self, index: usize) -> bool {
        self.violation.as_ref().map_or(false, |v| v.stopped) || (self.start_index > index)
    }
}

/// Provides the way to use copy on write strategy within route state context.
struct ShadowContext<'a> {
    is_mutated: bool,
    is_dirty: bool,
    constraint: &'a ConstraintPipeline,
    ctx: RouteContext,
}

impl<'a> ShadowContext<'a> {
    fn new(constraint: &'a ConstraintPipeline, ctx: &RouteContext) -> Self {
        Self { is_mutated: false, is_dirty: false, constraint, ctx: ctx.clone() }
    }

    fn insert(&mut self, activity: Activity, index: usize) -> Activity {
        if !self.is_mutated {
            self.ctx = self.ctx.deep_copy();
            self.is_mutated = true;
        }

        self.ctx.route_mut().tour.insert_at(activity.deep_copy(), index + 1);
        self.constraint.accept_route_state(&mut self.ctx);
        self.is_dirty = true;

        activity
    }

    fn restore(&mut self, original: &RouteContext) {
        if self.is_dirty {
            self.ctx = original.clone();
            self.is_mutated = false;
            self.is_dirty = false;
        }
    }
}

fn concat_activities(
    activities: Option<Vec<(Activity, usize)>>,
    activity: (Activity, usize),
) -> Vec<(Activity, usize)> {
    let mut activities = activities.unwrap_or_default();
    activities.push((activity.0, activity.1));

    activities
}
