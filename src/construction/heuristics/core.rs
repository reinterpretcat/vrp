use crate::construction::states::*;
use crate::models::common::{Cost, TimeWindow};
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Detail, Place};
use std::borrow::Borrow;
use std::sync::{Arc, Mutex};

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
                        _ => std::f64::MAX,
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
        route_context: &RouteContext,
        progress: &InsertionProgress,
    ) -> InsertionResult {
        let activity = Arc::new(Activity::new_with_job(job.clone()));
        let route_costs = ctx
            .problem
            .constraint
            .evaluate_soft_route(route_context, job);

        // 1. analyze route legs
        let result = unwrap_from_result(route_context.route.tour.legs().try_fold(
            SingleContext::new(progress.cost),
            |out, (items, index)| {
                let (prev, next) = match items {
                    [prev, next] => (prev.clone(), next.clone()),
                    _ => panic!("Unexpected route leg configuration."),
                };

                let activity_ctx = ActivityContext {
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
                        // TODO
                        //                        activity.lock().unwrap().place = Place {
                        //                            location: 0,//activity_ctx.prev.place.location,
                        //                            duration: detail.duration,
                        //                            time: time.clone()
                        //                        };

                        Result::Ok(in2)
                    })
                })
            },
        ));

        // TODO
        InsertionResult::make_failure()
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
    /// True, if processing has to be stopped.
    pub is_stopped: bool,
    /// Violation code.
    pub code: i32,
    /// Insertion index.
    pub index: usize,
    /// Best cost.
    pub cost: Cost,
    /// Activity detail.
    pub detail: Detail,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Cost) -> Self {
        Self {
            is_stopped: false,
            code: 0,
            index: 0,
            cost,
            detail: Detail {
                start: None,
                end: None,
                time: TimeWindow {
                    start: 0.0,
                    end: 0.0,
                },
            },
        }
    }
}

fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}
