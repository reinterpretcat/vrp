extern crate rand;

use std::borrow::Borrow;
use std::slice::Iter;
use std::sync::{Arc, RwLock};

use rand::seq::IteratorRandom;

use crate::construction::constraints::ActivityConstraintViolation;
use crate::construction::states::*;
use crate::models::common::{Cost, TimeWindow, NO_COST};
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Place, Route, TourActivity};
use crate::models::Problem;

#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/evaluators_test.rs"]
mod evaluators_test;

/// Provides the way to evaluate insertion cost.
pub struct InsertionEvaluator {}

impl InsertionEvaluator {
    pub fn new() -> Self {
        InsertionEvaluator {}
    }

    /// Evaluates possibility to preform insertion from given insertion context.
    pub fn evaluate(&self, job: &Arc<Job>, ctx: &InsertionContext) -> InsertionResult {
        ctx.solution.routes.iter().cloned().chain(ctx.solution.registry.next().map(|a| RouteContext::new(a))).fold(
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
                        InsertionResult::Success(success) => success.cost,
                        _ => NO_COST,
                    },
                    completeness: ctx.progress.completeness,
                    total: ctx.progress.total,
                };

                InsertionResult::choose_best_result(
                    acc,
                    match job.borrow() {
                        Job::Single(single) => Self::evaluate_single(job, single, ctx, &route_ctx, &progress),
                        Job::Multi(multi) => Self::evaluate_multi(job, multi, ctx, &route_ctx, &progress),
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
        let mut activity = Box::new(Activity::new_with_job(job.clone()));
        let route_costs = ctx.problem.constraint.evaluate_soft_route(route_ctx, job);

        // 1. analyze route legs
        let result = unwrap_from_result(route_ctx.route.read().unwrap().tour.legs().try_fold(
            SingleContext::new(progress.cost),
            |out, (items, index)| {
                let (prev, next) = match items {
                    [prev, next] => (prev.clone(), next.clone()),
                    _ => panic!("Unexpected route leg configuration."),
                };

                // 2. analyze service details
                single.places.iter().try_fold(out, |in1, detail| {
                    // TODO check whether tw is empty
                    // 3. analyze detail time windows
                    detail.times.iter().try_fold(in1, |in2, time| {
                        activity.place = Place {
                            location: detail.location.unwrap_or(prev.place.location),
                            duration: detail.duration,
                            time: time.clone(),
                        };

                        let activity_ctx = ActivityContext { index, prev, target: &activity, next: Some(next) };

                        if let Some(violation) = ctx.problem.constraint.evaluate_hard_activity(route_ctx, &activity_ctx)
                        {
                            return SingleContext::fail(violation, in2);
                        }

                        let total_costs = ctx.problem.constraint.evaluate_soft_activity(route_ctx, &activity_ctx);

                        if total_costs < in2.cost {
                            SingleContext::success(
                                activity_ctx.index,
                                total_costs,
                                Place {
                                    location: activity.place.location,
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
            activity.place = result.place;
            InsertionResult::make_success(result.cost, job.clone(), vec![(activity, result.index)], route_ctx.clone())
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
        //get_job_permutations(multi).iter().try_fold(|acc, services| {});

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
        Result::Ok(Self { violation: None, index, cost, place })
    }

    fn skip(other: SingleContext) -> Result<Self, Self> {
        Result::Ok(other)
    }

    fn is_success(&self) -> bool {
        self.cost < NO_COST
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
    pub activities: Vec<(TourActivity, usize)>,
}

impl MultiContext {
    /// Creates new empty insertion context.
    fn new() -> Self {
        Self { violation: None, start_index: 0, next_index: 0, cost: None, activities: vec![] }
    }

    /// Promotes insertion context by best price.
    fn promote(left: Self, right: Self) -> Self {
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
            _ => right,
        };

        Self {
            violation: best.violation,
            start_index: index,
            next_index: index,
            cost: best.cost,
            activities: best.activities,
        }
    }

    /// Creates failed insertion context within reason code.
    fn fail(err_ctx: SingleContext, other_ctx: MultiContext) -> Self {
        let violation = &err_ctx.violation.unwrap();
        let is_stopped = violation.stopped && other_ctx.activities.is_empty();

        Self {
            violation: Some(ActivityConstraintViolation { code: violation.code, stopped: is_stopped }),
            start_index: other_ctx.start_index,
            next_index: other_ctx.start_index,
            cost: None,
            activities: vec![],
        }
    }

    /// Creates successful insertion context.
    fn success(cost: Cost, activities: Vec<(TourActivity, usize)>) -> Self {
        Self {
            violation: None,
            start_index: activities.first().unwrap().1,
            next_index: activities.last().unwrap().1 + 1,
            cost: None,
            activities,
        }
    }

    /// Creates next insertion context from existing one.
    fn next(&self) -> Self {
        Self {
            violation: None,
            start_index: self.start_index,
            next_index: self.start_index,
            cost: None,
            activities: vec![],
        }
    }

    /// Checks whether insertion is found.
    fn is_success(&self) -> bool {
        self.violation.is_none() & self.cost.is_some()
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
    fn insert(&mut self, activity: &TourActivity, index: usize) {
        if !self.is_mutated {
            self.ctx = self.ctx.deep_copy();
            self.is_mutated = true;
        }
    }
}

fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}

fn get_job_permutations(multi: &Multi) -> Vec<Vec<&Single>> {
    // TODO optionally use permutation function defined on multi job
    // TODO configure sample size
    // TODO avoid extra memory allocations?
    const SAMPLE_SIZE: usize = 3;

    let mut rng = rand::thread_rng();
    get_permutations(multi.jobs.len())
        .choose_multiple(&mut rng, SAMPLE_SIZE)
        .iter()
        .map(|permutation| permutation.iter().map(|&i| multi.jobs.get(i).unwrap()).collect::<Vec<&Single>>())
        .collect()
}

fn get_permutations(size: usize) -> Permutations {
    Permutations { idxs: (0..size).collect(), swaps: vec![0; size], i: 0 }
}

struct Permutations {
    idxs: Vec<usize>,
    swaps: Vec<usize>,
    i: usize,
}

impl Iterator for Permutations {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i > 0 {
            loop {
                if self.i >= self.swaps.len() {
                    return None;
                }
                if self.swaps[self.i] < self.i {
                    break;
                }
                self.swaps[self.i] = 0;
                self.i += 1;
            }
            self.idxs.swap(self.i, (self.i & 1) * self.swaps[self.i]);
            self.swaps[self.i] += 1;
        }
        self.i = 1;
        Some(self.idxs.clone())
    }
}
