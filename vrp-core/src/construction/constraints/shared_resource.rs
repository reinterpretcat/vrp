#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/shared_resource_test.rs"]
mod shared_resource_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::common::{MultiDimLoad, SingleDimLoad};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route};
use hashbrown::HashMap;
use std::ops::{Add, Deref, RangeInclusive, Sub};
use std::slice::Iter;
use std::sync::Arc;

/// Represents a shared unique resource.
pub trait SharedResource: Add + Sub + Ord + Copy + Sized + Send + Sync + Default + 'static {}
/// Represents a shared resource id.
pub type SharedResourceId = usize;
/// Specifies a type for a shared resource interval function.
pub type SharedResourceIntervalFn = Arc<dyn Fn(&RouteContext) -> Option<&Vec<(usize, usize)>> + Send + Sync>;
/// Specifies a type for a shared resource capacity function.
pub type SharedResourceCapacityFn<T> = Arc<dyn Fn(&Activity) -> Option<(T, SharedResourceId)> + Send + Sync>;
/// Specifies a type for a shared resource demand function.
pub type SharedResourceDemandFn<T> = Arc<dyn Fn(&Single) -> Option<T> + Send + Sync>;

/// Provides way to define and use time independent, shared across multiple routes resource.
pub struct SharedResourceModule<T>
where
    T: SharedResource + Add<Output = T> + Sub<Output = T>,
{
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    interval_fn: SharedResourceIntervalFn,
    resource_capacity_fn: SharedResourceCapacityFn<T>,
    resource_demand_fn: SharedResourceDemandFn<T>,
    total_jobs: usize,
    resource_key: i32,
}

impl<T: SharedResource + Add<Output = T> + Sub<Output = T>> SharedResourceModule<T> {
    /// Creates a new instance of `SharedResourceModule`.
    pub fn new(
        total_jobs: usize,
        code: i32,
        resource_key: i32,
        interval_fn: SharedResourceIntervalFn,
        resource_capacity_fn: SharedResourceCapacityFn<T>,
        resource_demand_fn: SharedResourceDemandFn<T>,
    ) -> Self {
        Self {
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(SharedResourceHardRouteConstraint {
                    code,
                    total_jobs,
                    interval_fn: interval_fn.clone(),
                    resource_demand_fn: resource_demand_fn.clone(),
                })),
                ConstraintVariant::HardActivity(Arc::new(SharedResourceHardActivityConstraint {
                    code,
                    interval_fn: interval_fn.clone(),
                    resource_demand_fn: resource_demand_fn.clone(),
                    resource_key,
                })),
            ],
            interval_fn,
            resource_demand_fn,
            state_keys: vec![resource_key],
            resource_capacity_fn,
            total_jobs,
            resource_key,
        }
    }

    /// Calculates available resource based on consumption in the whole solution.
    fn update_resource_consumption(&self, solution_ctx: &mut SolutionContext) {
        // NOTE: we cannot estimate resource consumption in partial solutions
        if solution_ctx.get_jobs_amount() != self.total_jobs {
            return;
        }

        // first pass: get total demand for each shared resource
        let total_demand = solution_ctx.routes.iter().fold(HashMap::<usize, T>::default(), |acc, route_ctx| {
            self.interval_fn.deref()(route_ctx).iter().flat_map(|intervals| intervals.iter()).fold(
                acc,
                |mut acc, &(start_idx, end_idx)| {
                    // get total resource demand for given interval
                    let activity = get_activity_by_idx(&route_ctx.route, start_idx);
                    let resource_demand_with_id = self.resource_capacity_fn.deref()(activity)
                        .map(|(_, resource_id)| (self.get_total_demand(route_ctx, start_idx..=end_idx), resource_id));

                    if let Some((resource_demand, id)) = resource_demand_with_id {
                        let entry = acc.entry(id).or_insert_with(T::default);
                        *entry = *entry + resource_demand;
                    }

                    acc
                },
            )
        });

        // second pass: store amount of available resources inside activity state
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            #[allow(clippy::unnecessary_to_owned)]
            self.interval_fn.deref()(route_ctx).cloned().unwrap_or_default().into_iter().for_each(|(start_idx, _)| {
                let resource_available =
                    self.resource_capacity_fn.deref()(get_activity_by_idx(&route_ctx.route, start_idx)).and_then(
                        |(total_capacity, resource_id)| {
                            total_demand.get(&resource_id).map(|total_demand| total_capacity - *total_demand)
                        },
                    );

                if let Some(resource_available) = resource_available {
                    let (route, state) = route_ctx.as_mut();
                    state.put_activity_state(
                        self.resource_key,
                        get_activity_by_idx(route, start_idx),
                        resource_available,
                    );
                }
            });
        });
    }

    /// Prevents resource consumption in given route by setting available to zero (default).
    fn prevent_resource_consumption(&self, route_ctx: &mut RouteContext) {
        self.interval_fn.deref()(route_ctx).cloned().unwrap_or_default().into_iter().for_each(
            |(start_idx, end_idx)| {
                let activity = get_activity_by_idx(&route_ctx.route, start_idx);
                let has_resource_demand = self.resource_capacity_fn.deref()(activity).map_or(false, |(_, _)| {
                    (start_idx..=end_idx)
                        .into_iter()
                        .filter_map(|idx| route_ctx.route.tour.get(idx))
                        .filter_map(|activity| activity.job.as_ref())
                        .any(|job| self.resource_demand_fn.deref()(job).is_some())
                });

                if has_resource_demand {
                    let (route, state) = route_ctx.as_mut();
                    state.put_activity_state(self.resource_key, get_activity_by_idx(route, start_idx), T::default());
                }
            },
        );
    }

    fn get_total_demand(&self, route_ctx: &RouteContext, range: RangeInclusive<usize>) -> T {
        range
            .into_iter()
            .filter_map(|idx| route_ctx.route.tour.get(idx))
            .filter_map(|activity| activity.job.as_ref())
            .fold(T::default(), |acc, job| acc + self.resource_demand_fn.deref()(job).unwrap_or_default())
    }
}

impl<T: SharedResource + Add<Output = T> + Sub<Output = T>> ConstraintModule for SharedResourceModule<T> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
        self.update_resource_consumption(solution_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        // NOTE: we need to prevent any insertions with resource consumption in modified route.
        //       This state will be overridden by update_resource_consumption after other accept
        //       method calls.

        self.prevent_resource_consumption(route_ctx);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        self.update_resource_consumption(solution_ctx);
    }

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct SharedResourceHardRouteConstraint<T: SharedResource> {
    code: i32,
    total_jobs: usize,
    interval_fn: SharedResourceIntervalFn,
    resource_demand_fn: SharedResourceDemandFn<T>,
}

impl<T: SharedResource> HardRouteConstraint for SharedResourceHardRouteConstraint<T> {
    fn evaluate_job(
        &self,
        solution_ctx: &SolutionContext,
        route_ctx: &RouteContext,
        job: &Job,
    ) -> Option<RouteConstraintViolation> {
        job.as_single()
            .and_then(|job| {
                self.resource_demand_fn.deref()(job)
                    .zip(self.interval_fn.deref()(route_ctx).and_then(|intervals| intervals.first()))
            })
            .and_then(|(_, _)| {
                // NOTE cannot do resource assignment for partial solution
                if solution_ctx.get_jobs_amount() != self.total_jobs {
                    Some(RouteConstraintViolation { code: self.code })
                } else {
                    None
                }
            })
    }
}

struct SharedResourceHardActivityConstraint<T: SharedResource> {
    code: i32,
    interval_fn: SharedResourceIntervalFn,
    resource_demand_fn: SharedResourceDemandFn<T>,
    resource_key: i32,
}

impl<T: SharedResource> HardActivityConstraint for SharedResourceHardActivityConstraint<T> {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        self.interval_fn.deref()(route_ctx)
            .iter()
            .flat_map(|intervals| intervals.iter())
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .and_then(|&(start_idx, _)| {
                route_ctx
                    .state
                    .get_activity_state::<T>(self.resource_key, get_activity_by_idx(&route_ctx.route, start_idx))
                    .and_then(|resource_available| {
                        let resource_demand = activity_ctx
                            .target
                            .job
                            .as_ref()
                            .and_then(|job| self.resource_demand_fn.deref()(job.as_ref()))
                            .unwrap_or_default();

                        if resource_available < &resource_demand {
                            Some(ActivityConstraintViolation { code: self.code, stopped: false })
                        } else {
                            None
                        }
                    })
            })
    }
}

fn get_activity_by_idx(route: &Route, idx: usize) -> &Activity {
    route.tour.get(idx).expect("cannot get activity by idx")
}

/// Implement SharedResource for multi dimensional load.
impl SharedResource for MultiDimLoad {}

/// Implement SharedResource for single dimensional load.
impl SharedResource for SingleDimLoad {}
