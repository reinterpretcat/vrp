use crate::construction::constraints::ActivityConstraintViolation;
use crate::construction::states::*;
use crate::models::common::{Cost, TimeWindow, NO_COST};
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Place};
use std::borrow::Borrow;
use std::sync::{Arc, RwLock};

/// Provides the way to evaluate insertion cost.
pub struct InsertionEvaluator {}

impl InsertionEvaluator {
    pub fn new() -> Self {
        InsertionEvaluator {}
    }

    /// Evaluates possibility to preform insertion from given insertion context.
    pub fn evaluate(&self, job: &Arc<Job>, ctx: &InsertionContext) -> InsertionResult {
        ctx.solution
            .routes
            .iter()
            .cloned()
            .chain(ctx.solution.registry.next().map(|a| RouteContext::new(a)))
            .fold(InsertionResult::make_failure(), |acc, route_ctx| {
                if let Some(violation) = ctx.problem.constraint.evaluate_hard_route(&route_ctx, job)
                {
                    return InsertionResult::choose_best_result(
                        acc,
                        InsertionResult::make_failure_with_code(violation.code),
                    );
                }

                let progress = InsertionProgress {
                    cost: match acc.borrow() {
                        InsertionResult::Success(success) => success.cost,
                        _ => NO_COST,
                    },
                    completeness: ctx.progress.completeness,
                    total: ctx.progress.total,
                };

                InsertionResult::choose_best_result(
                    acc,
                    match job.borrow() {
                        Job::Single(single) => {
                            Self::evaluate_single(job, single, ctx, &route_ctx, &progress)
                        }
                        Job::Multi(multi) => {
                            Self::evaluate_multi(job, multi, ctx, &route_ctx, &progress)
                        }
                    },
                )
            })
    }

    fn evaluate_single(
        job: &Arc<Job>,
        single: &Single,
        ctx: &InsertionContext,
        route_ctx: &RouteContext,
        progress: &InsertionProgress,
    ) -> InsertionResult {
        let activity = Arc::new(RwLock::new(Activity::new_with_job(job.clone())));
        let route_costs = ctx.problem.constraint.evaluate_soft_route(route_ctx, job);

        // 1. analyze route legs
        let result = unwrap_from_result(route_ctx.route.tour.legs().try_fold(
            SingleContext::new(progress.cost),
            |out, (items, index)| {
                let (prev, next) = match items {
                    [prev, next] => (prev.clone(), next.clone()),
                    _ => panic!("Unexpected route leg configuration."),
                };

                let mut activity_ctx = ActivityContext {
                    index,
                    prev,
                    target: activity.clone(),
                    next: Some(next),
                };

                // 2. analyze service details
                single.places.iter().try_fold(out, |in1, detail| {
                    // TODO check whether tw is empty
                    // 3. analyze detail time windows
                    detail.times.iter().try_fold(in1, |in2, time| {
                        activity.write().unwrap().place = Place {
                            location: detail
                                .location
                                .unwrap_or(activity_ctx.prev.read().unwrap().place.location),
                            duration: detail.duration,
                            time: time.clone(),
                        };

                        if let Some(violation) = ctx
                            .problem
                            .constraint
                            .evaluate_hard_activity(route_ctx, &activity_ctx)
                        {
                            return SingleContext::fail(violation, in2);
                        }

                        let total_costs = ctx
                            .problem
                            .constraint
                            .evaluate_soft_activity(route_ctx, &activity_ctx);

                        if total_costs < in2.cost {
                            SingleContext::success(
                                activity_ctx.index,
                                total_costs,
                                Place {
                                    location: activity.read().unwrap().place.location,
                                    duration: detail.duration,
                                    time: time.clone(),
                                },
                            )
                        } else {
                            SingleContext::skip(in2)
                        }
                    })
                })
            },
        ));

        if result.is_success() {
            activity.write().unwrap().place = result.place;
            InsertionResult::make_success(
                result.cost,
                job.clone(),
                vec![(activity, result.index)],
                route_ctx.clone(),
            )
        } else {
            InsertionResult::make_failure_with_code(result.violation.unwrap().code)
        }
    }

    fn evaluate_multi(
        job: &Arc<Job>,
        multi: &Multi,
        ctx: &InsertionContext,
        route_context: &RouteContext,
        progress: &InsertionProgress,
    ) -> InsertionResult {
        unimplemented!()
    }
}

/// Stores information needed for single insertion.
struct SingleContext {
    /// Constraint violation.
    pub violation: Option<ActivityConstraintViolation>,
    /// Insertion index.
    pub index: usize,
    /// Best cost.
    pub cost: Cost,
    /// Activity place.
    pub place: Place,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Cost) -> Self {
        Self {
            violation: None,
            index: 0,
            cost,
            place: Place {
                location: 0,
                duration: 0.0,
                time: TimeWindow {
                    start: 0.0,
                    end: 0.0,
                },
            },
        }
    }

    fn fail(violation: ActivityConstraintViolation, other: SingleContext) -> Result<Self, Self> {
        let stopped = violation.stopped;
        let ctx = Self {
            violation: Some(violation),
            index: other.index,
            cost: other.cost,
            place: other.place,
        };
        if stopped {
            Result::Err(ctx)
        } else {
            Result::Ok(ctx)
        }
    }

    fn success(index: usize, cost: Cost, place: Place) -> Result<Self, Self> {
        Result::Ok(Self {
            violation: None,
            index,
            cost,
            place,
        })
    }

    fn skip(other: SingleContext) -> Result<Self, Self> {
        Result::Ok(other)
    }

    fn is_success(&self) -> bool {
        self.cost < NO_COST
    }
}

fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}
