#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/evaluators_test.rs"]
mod evaluators_test;

use std::borrow::Borrow;
use std::sync::Arc;

use crate::construction::constraints::ActivityConstraintViolation;
use crate::construction::states::*;
use crate::models::common::{Cost, TimeWindow};
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Place, TourActivity};
use crate::models::Problem;

/// Evaluates possibility to preform insertion from given insertion context.
pub fn evaluate_job_insertion(job: &Arc<Job>, ctx: &InsertionContext) -> InsertionResult {
    ctx.solution.routes.iter().cloned().chain(ctx.solution.registry.next().map(RouteContext::new)).fold(
        InsertionResult::make_failure(),
        |acc, route_ctx| {
            if let Some(violation) = ctx.problem.constraint.evaluate_hard_route(&route_ctx, job) {
                return InsertionResult::choose_best_result(
                    acc,
                    InsertionResult::make_failure_with_code(violation.code),
                );
            }

            let progress = InsertionProgress {
                cost: match acc.borrow() {
                    InsertionResult::Success(success) => Some(success.cost),
                    _ => None,
                },
                completeness: ctx.progress.completeness,
                total: ctx.progress.total,
            };

            InsertionResult::choose_best_result(
                acc,
                match job.borrow() {
                    Job::Single(single) => evaluate_single(job, single, ctx, &route_ctx, &progress),
                    Job::Multi(multi) => evaluate_multi(job, multi, ctx, &route_ctx, &progress),
                },
            )
        },
    )
}

fn evaluate_single(
    job: &Arc<Job>,
    single: &Single,
    ctx: &InsertionContext,
    route_ctx: &RouteContext,
    progress: &InsertionProgress,
) -> InsertionResult {
    let route_costs = ctx.problem.constraint.evaluate_soft_route(route_ctx, job);
    let mut activity = Box::new(Activity::new_with_job(job.clone()));
    let result = analyze_insertion_in_route(
        ctx,
        route_ctx,
        single,
        &mut activity,
        route_costs,
        SingleContext::new(progress.cost, 0),
    );

    if result.is_success() {
        activity.place = result.place;
        let activities = vec![(activity, result.index)];
        InsertionResult::make_success(result.cost.unwrap(), job.clone(), activities, route_ctx.clone())
    } else {
        InsertionResult::make_failure_with_code(result.violation.unwrap().code)
    }
}

fn evaluate_multi(
    job: &Arc<Job>,
    multi: &Multi,
    ctx: &InsertionContext,
    route_ctx: &RouteContext,
    progress: &InsertionProgress,
) -> InsertionResult {
    let route_costs = ctx.problem.constraint.evaluate_soft_route(route_ctx, job);
    // 1. analyze permutations
    let result = unwrap_from_result(multi.permutations().into_iter().try_fold(
        MultiContext::new(progress.cost),
        |acc_res, services| {
            let mut shadow = ShadowContext::new(&ctx.problem, &route_ctx);
            let perm_res = unwrap_from_result(std::iter::repeat(0).try_fold(MultiContext::new(None), |out, _| {
                {
                    let route = route_ctx.route.read().unwrap();
                    if out.is_failure(route.tour.activity_count()) {
                        return Result::Err(out);
                    }
                }
                shadow.restore(job);

                // 2. analyze inner jobs
                let sq_res = unwrap_from_result(services.iter().try_fold(out.next(), |in1, service| {
                    if in1.violation.is_some() {
                        return Result::Err(in1);
                    }
                    let mut activity = Box::new(Activity::new_with_job(Arc::new(Job::Single(service.clone()))));
                    // 3. analyze legs
                    let srv_res = analyze_insertion_in_route(
                        ctx,
                        &shadow.ctx,
                        service,
                        &mut activity,
                        0.0,
                        SingleContext::new(None, in1.next_index),
                    );

                    if srv_res.is_success() {
                        activity.place = srv_res.place;
                        let activity = shadow.insert(activity, srv_res.index);
                        let activities = concat_activities(in1.activities, (activity, srv_res.index));
                        return MultiContext::success(
                            in1.cost.unwrap_or(route_costs) + srv_res.cost.unwrap(),
                            activities,
                        );
                    }

                    MultiContext::fail(srv_res, in1)
                }));

                MultiContext::promote(sq_res, out)
            }));

            MultiContext::promote(perm_res, acc_res)
        },
    ));

    if result.is_success() {
        let activities = result.activities.unwrap();
        InsertionResult::make_success(result.cost.unwrap(), job.clone(), activities, route_ctx.clone())
    } else {
        InsertionResult::make_failure_with_code(if let Some(violation) = result.violation { violation.code } else { 0 })
    }
}

#[inline(always)]
fn analyze_insertion_in_route(
    ctx: &InsertionContext,
    route_ctx: &RouteContext,
    single: &Single,
    target: &mut Box<Activity>,
    extra_costs: Cost,
    init: SingleContext,
) -> SingleContext {
    unwrap_from_result(route_ctx.route.read().unwrap().tour.legs().skip(init.index).try_fold(
        init,
        |out, (items, index)| {
            let (prev, next) = match items {
                [prev, next] => (prev, next),
                _ => panic!("Unexpected route leg configuration."),
            };
            // analyze service details
            single.places.iter().try_fold(out, |in1, detail| {
                // analyze detail time windows
                detail.times.iter().try_fold(in1, |in2, time| {
                    target.place = Place {
                        location: detail.location.unwrap_or(prev.place.location),
                        duration: detail.duration,
                        time: time.clone(),
                    };

                    let activity_ctx = ActivityContext { index, prev, target: &target, next: Some(next) };

                    if let Some(violation) = ctx.problem.constraint.evaluate_hard_activity(route_ctx, &activity_ctx) {
                        return SingleContext::fail(violation, in2);
                    }

                    let costs = extra_costs + ctx.problem.constraint.evaluate_soft_activity(route_ctx, &activity_ctx);

                    if costs < in2.cost.unwrap_or(std::f64::MAX) {
                        SingleContext::success(activity_ctx.index, costs, target.place.clone())
                    } else {
                        SingleContext::skip(in2)
                    }
                })
            })
        },
    ))
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
    pub place: Place,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Option<Cost>, index: usize) -> Self {
        Self {
            violation: None,
            index,
            cost,
            place: Place { location: 0, duration: 0.0, time: TimeWindow { start: 0.0, end: 0.0 } },
        }
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

    fn success(index: usize, cost: Cost, place: Place) -> Result<Self, Self> {
        Result::Ok(Self { violation: None, index, cost: Some(cost), place })
    }

    fn skip(other: SingleContext) -> Result<Self, Self> {
        Result::Ok(other)
    }

    fn is_success(&self) -> bool {
        self.cost.is_some()
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
    pub activities: Option<Vec<(TourActivity, usize)>>,
}

impl MultiContext {
    /// Creates new empty insertion context.
    fn new(cost: Option<Cost>) -> Self {
        Self { violation: None, start_index: 0, next_index: 0, cost, activities: None }
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
        let violation = &err_ctx.violation.unwrap();
        let is_stopped = violation.stopped && other_ctx.activities.is_none();

        Result::Err(Self {
            violation: Some(ActivityConstraintViolation { code: violation.code, stopped: is_stopped }),
            start_index: other_ctx.start_index,
            next_index: other_ctx.start_index,
            cost: None,
            activities: None,
        })
    }

    /// Creates successful insertion context.
    fn success(cost: Cost, activities: Vec<(TourActivity, usize)>) -> Result<Self, Self> {
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
struct ShadowContext {
    is_mutated: bool,
    is_dirty: bool,
    problem: Arc<Problem>,
    ctx: RouteContext,
}

impl ShadowContext {
    fn new(problem: &Arc<Problem>, ctx: &RouteContext) -> Self {
        Self {
            is_mutated: false,
            is_dirty: false,
            problem: problem.clone(),
            ctx: RouteContext { route: ctx.route.clone(), state: ctx.state.clone() },
        }
    }

    fn insert(&mut self, activity: TourActivity, index: usize) -> TourActivity {
        if !self.is_mutated {
            self.ctx = self.ctx.deep_copy();
            self.is_mutated = true;
        }
        {
            let mut route = self.ctx.route.write().unwrap();
            route.tour.insert_at(activity, index + 1);
        }

        {
            self.problem.constraint.accept_route_state(&mut self.ctx);
            self.is_dirty = true;
        }

        Box::new(self.ctx.route.read().unwrap().tour.get(index + 1).unwrap().deep_copy())
    }

    fn restore(&mut self, job: &Arc<Job>) {
        if self.is_dirty {
            {
                let mut state = self.ctx.state.write().unwrap();
                let mut route = self.ctx.route.write().unwrap();
                route.tour.all_activities().for_each(|a| state.remove_activity_states(a));
                route.tour.remove(job);
            }
            self.problem.constraint.accept_route_state(&mut self.ctx);
        }
    }
}

fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}

fn concat_activities(
    activities: Option<Vec<(Box<Activity>, usize)>>,
    activity: (Box<Activity>, usize),
) -> Vec<(Box<Activity>, usize)> {
    let mut activities = activities.unwrap_or_else(|| vec![]);
    activities.push((activity.0, activity.1));

    activities
}
