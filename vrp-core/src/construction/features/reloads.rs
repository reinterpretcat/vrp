//! This module provides functionality for reloading vehicle with new jobs at some place later in
//! the tour. This is used to overcome a vehicle capacity limit. The feature has two flavors:
//!  - simple: a basic reload place with unlimited number of jobs which can be loaded/unloaded from there
//!  - shared: a resource constrained reload place

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/reloads_test.rs"]
mod reloads_test;

use crate::construction::enablers::{FeatureCombinator, RouteIntervals, RouteIntervalsState};
use crate::construction::features::capacity::*;
use crate::construction::heuristics::*;
use crate::models::common::{Demand, LoadOps, MultiDimLoad, SingleDimLoad};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Route};
use crate::models::*;
use rosomaxa::utils::{GenericError, GenericResult};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::{Add, Range, RangeInclusive, Sub};
use std::sync::Arc;

/// Represents a shared unique resource which is used to model reload with capacity constraint.
pub trait SharedResource: LoadOps + Add + Sub + PartialOrd + Copy + Sized + Send + Sync + Default + 'static {}

/// Represents a shared resource id.
pub type SharedResourceId = usize;

/// Provides a way to build reload feature with various parameters.
#[allow(clippy::type_complexity)]
pub struct ReloadFeatureFactory<T: LoadOps> {
    name: String,
    capacity_code: Option<ViolationCode>,
    resource_code: Option<ViolationCode>,

    is_reload_single_fn: Option<Arc<dyn Fn(&Single) -> bool + Send + Sync>>,
    belongs_to_route_fn: Option<Arc<dyn Fn(&Route, &Job) -> bool + Send + Sync>>,
    load_schedule_threshold_fn: Option<Box<dyn Fn(&T) -> T + Send + Sync>>,

    // these fields are needed to be set for shared reload flavor
    shared_resource_capacity_fn: Option<SharedResourceCapacityFn<T>>,
    shared_resource_demand_fn: Option<SharedResourceDemandFn<T>>,
    is_partial_solution_fn: Option<PartialSolutionFn>,
}

impl<T: SharedResource> ReloadFeatureFactory<T> {
    /// Sets resource constraint violation code which is used to report back the reason of job's unassignment.
    pub fn set_resource_code(mut self, code: ViolationCode) -> Self {
        self.resource_code = Some(code);
        self
    }

    /// Sets a shared resource capacity function.
    pub fn set_shared_resource_capacity<F>(mut self, func: F) -> Self
    where
        F: Fn(&Activity) -> Option<(T, SharedResourceId)> + Send + Sync + 'static,
    {
        self.shared_resource_capacity_fn = Some(Arc::new(func));
        self
    }

    /// Sets a shared resource demand function.
    pub fn set_shared_demand_capacity<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> Option<T> + Send + Sync + 'static,
    {
        self.shared_resource_demand_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which tells whether a given solution is partial.
    pub fn set_is_partial_solution<F>(mut self, func: F) -> Self
    where
        F: Fn(&SolutionContext) -> bool + Send + Sync + 'static,
    {
        self.is_partial_solution_fn = Some(Arc::new(func));
        self
    }

    /// Builds a shared reload flavor.
    pub fn build_shared(mut self) -> GenericResult<Feature> {
        let violation_code = self.resource_code.unwrap_or_default();

        // read shared resource flavor properties
        let Some(((resource_capacity_fn, resource_demand_fn), is_partial_solution_fn)) = self
            .shared_resource_capacity_fn
            .take()
            .zip(self.shared_resource_demand_fn.take())
            .zip(self.is_partial_solution_fn.take())
        else {
            return Err("shared_resource_capacity, shared_resource_demand and partial_solution must be set for shared reload feature".into());
        };

        let shared_resource_threshold_fn: SharedResourceThresholdFn<T> =
            Box::new(move |route_ctx: &RouteContext, activity_idx, demand| {
                route_ctx
                    .state()
                    .get_activity_state::<SharedResourceStateKey, T>(activity_idx)
                    .map_or(true, |resource_available| resource_available.can_fit(demand))
            });

        let simple_reload = self.build(Some(shared_resource_threshold_fn))?;

        let shared_resource = FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_constraint(SharedResourceConstraint {
                violation_code,
                resource_demand_fn: resource_demand_fn.clone(),
                is_partial_solution_fn: is_partial_solution_fn.clone(),
            })
            .with_state(SharedResourceState { resource_capacity_fn, resource_demand_fn, is_partial_solution_fn })
            .build()?;

        FeatureCombinator::default().use_name(self.name).add_features(&[simple_reload, shared_resource]).combine()
    }
}

impl<T: LoadOps> ReloadFeatureFactory<T> {
    /// Creates a new instance of `ReloadFeatureFactory` with the given feature name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            capacity_code: None,
            resource_code: None,
            is_reload_single_fn: None,
            belongs_to_route_fn: None,
            load_schedule_threshold_fn: None,
            shared_resource_capacity_fn: None,
            shared_resource_demand_fn: None,
            is_partial_solution_fn: None,
        }
    }

    /// Sets capacity constraint violation code which is used to report back the reason of job's unassignment.
    pub fn set_capacity_code(mut self, code: ViolationCode) -> Self {
        self.capacity_code = Some(code);
        self
    }

    /// Sets a function which specifies whether a given single job can be considered as a reload job.
    pub fn set_is_reload_single<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> bool + Send + Sync + 'static,
    {
        self.is_reload_single_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which specifies whether a given route can serve a given job. This function
    /// should return false, if the job is not reload.
    pub fn set_belongs_to_route<F>(mut self, func: F) -> Self
    where
        F: Fn(&Route, &Job) -> bool + Send + Sync + 'static,
    {
        self.belongs_to_route_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which is used to decide whether reload should be considered for assignment
    /// based on the left vehicle's capacity.
    pub fn set_load_schedule_threshold<F>(mut self, func: F) -> Self
    where
        F: Fn(&T) -> T + Send + Sync + 'static,
    {
        self.load_schedule_threshold_fn = Some(Box::new(func));
        self
    }

    /// Builds a simple reload flavor.
    pub fn build_simple(mut self) -> GenericResult<Feature> {
        self.build(None)
    }

    fn build(&mut self, shared_resource_threshold_fn: Option<SharedResourceThresholdFn<T>>) -> GenericResult<Feature> {
        // TODO provide reasonable default to simplify code usage?

        // read common properties
        let is_marker_single_fn =
            self.is_reload_single_fn.take().ok_or_else(|| GenericError::from("is_reload_single must be set"))?;
        let is_assignable_fn =
            self.belongs_to_route_fn.take().ok_or_else(|| GenericError::from("belongs_to_route must be set"))?;
        let load_schedule_threshold_fn = self
            .load_schedule_threshold_fn
            .take()
            .ok_or_else(|| GenericError::from("load_schedule_threshold must be set"))?;

        // create route intervals used to control how tour is split into multiple sub-tours
        let route_intervals = RouteIntervals::Multiple {
            is_marker_single_fn,
            is_new_interval_needed_fn: Arc::new(move |route_ctx| {
                route_ctx
                    .route()
                    .tour
                    .end_idx()
                    .map(|end_idx| {
                        let current: T =
                            route_ctx.state().get_max_past_capacity_at(end_idx).cloned().unwrap_or_default();

                        let max_capacity =
                            route_ctx.route().actor.vehicle.dimens.get_vehicle_capacity().cloned().unwrap_or_default();
                        let threshold_capacity = (load_schedule_threshold_fn)(&max_capacity);

                        current.partial_cmp(&threshold_capacity) != Some(Ordering::Less)
                    })
                    .unwrap_or(false)
            }),
            is_obsolete_interval_fn: Arc::new(move |route_ctx, left, right| {
                let capacity: T =
                    route_ctx.route().actor.vehicle.dimens.get_vehicle_capacity().cloned().unwrap_or_default();

                let fold_demand = |range: Range<usize>, demand_fn: fn(&Demand<T>) -> T| {
                    route_ctx.route().tour.activities_slice(range.start, range.end).iter().fold(
                        T::default(),
                        |acc, activity| {
                            activity
                                .job
                                .as_ref()
                                .and_then(|job| job.dimens.get_job_demand())
                                .map(|demand| acc + demand_fn(demand))
                                .unwrap_or_else(|| acc)
                        },
                    )
                };

                let left_pickup = fold_demand(left.clone(), |demand| demand.pickup.0);
                let right_delivery = fold_demand(right.clone(), |demand| demand.delivery.0);

                // static delivery moved to left
                let new_max_load_left =
                    route_ctx.state().get_max_future_capacity_at::<T>(left.start).cloned().unwrap_or_default()
                        + right_delivery;

                // static pickup moved to right
                let new_max_load_right =
                    route_ctx.state().get_max_future_capacity_at::<T>(right.start).cloned().unwrap_or_default()
                        + left_pickup;

                let has_enough_vehicle_capacity =
                    capacity.can_fit(&new_max_load_left) && capacity.can_fit(&new_max_load_right);

                has_enough_vehicle_capacity
                    && shared_resource_threshold_fn.as_ref().map_or(true, |shared_resource_threshold_fn| {
                        // total static delivery at left
                        let left_delivery = fold_demand(left.start..right.end, |demand| demand.delivery.0);

                        (shared_resource_threshold_fn)(route_ctx, left.start, &left_delivery)
                    })
            }),
            is_assignable_fn,
            intervals_state: Arc::new(ReloadIntervalsState),
        };

        let violation_code = self.capacity_code.unwrap_or_default();

        // NOTE: all reload feature flavors extend the capacity feature via route intervals
        create_capacity_limit_with_multi_trip_feature::<T>(self.name.as_str(), route_intervals, violation_code)
    }
}

custom_route_intervals_state!(pub ReloadIntervals);

// shared reload implementation

// TODO: dedicated macro doesn't support Option<T> to be stored as a type
struct SharedResourceStateKey;

type SharedResourceCapacityFn<T> = Arc<dyn Fn(&Activity) -> Option<(T, SharedResourceId)> + Send + Sync>;
type SharedResourceDemandFn<T> = Arc<dyn Fn(&Single) -> Option<T> + Send + Sync>;
type SharedResourceThresholdFn<T> = Box<dyn Fn(&RouteContext, usize, &T) -> bool + Send + Sync>;
type PartialSolutionFn = Arc<dyn Fn(&SolutionContext) -> bool + Send + Sync>;

struct SharedResourceConstraint<T: SharedResource> {
    violation_code: ViolationCode,
    resource_demand_fn: SharedResourceDemandFn<T>,
    is_partial_solution_fn: PartialSolutionFn,
}

impl<T: SharedResource> SharedResourceConstraint<T> {
    fn evaluate_route(
        &self,
        solution_ctx: &SolutionContext,
        route_ctx: &RouteContext,
        job: &Job,
    ) -> Option<ConstraintViolation> {
        job.as_single()
            .and_then(|job| {
                (self.resource_demand_fn)(job)
                    .zip(route_ctx.state().get_reload_intervals().and_then(|intervals| intervals.first()))
            })
            .and_then(|(_, _)| {
                // NOTE cannot do resource assignment for partial solution
                if (self.is_partial_solution_fn)(solution_ctx) {
                    ConstraintViolation::fail(self.violation_code)
                } else {
                    ConstraintViolation::success()
                }
            })
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        route_ctx
            .state()
            .get_reload_intervals()
            .iter()
            .flat_map(|intervals| intervals.iter())
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .and_then(|&(start_idx, _)| {
                route_ctx
                    .state()
                    .get_activity_state::<SharedResourceStateKey, Option<T>>(start_idx)
                    .and_then(|resource_available| *resource_available)
                    .and_then(|resource_available| {
                        let resource_demand = activity_ctx
                            .target
                            .job
                            .as_ref()
                            .and_then(|job| (self.resource_demand_fn)(job.as_ref()))
                            .unwrap_or_default();

                        if resource_available
                            .partial_cmp(&resource_demand)
                            .map_or(false, |ordering| ordering == Ordering::Less)
                        {
                            ConstraintViolation::skip(self.violation_code)
                        } else {
                            ConstraintViolation::success()
                        }
                    })
            })
    }
}

impl<T: SharedResource> FeatureConstraint for SharedResourceConstraint<T> {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => self.evaluate_route(solution_ctx, route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

struct SharedResourceState<T>
where
    T: SharedResource + Add<Output = T> + Sub<Output = T>,
{
    resource_capacity_fn: SharedResourceCapacityFn<T>,
    resource_demand_fn: SharedResourceDemandFn<T>,
    is_partial_solution_fn: PartialSolutionFn,
}

impl<T: SharedResource + Add<Output = T> + Sub<Output = T>> SharedResourceState<T> {
    /// Calculates available resource based on consumption in the whole solution.
    fn update_resource_consumption(&self, solution_ctx: &mut SolutionContext) {
        // NOTE: we cannot estimate resource consumption in partial solutions
        if (self.is_partial_solution_fn)(solution_ctx) {
            return;
        }

        // first pass: get total demand for each shared resource
        let total_demand = solution_ctx.routes.iter().fold(HashMap::<usize, T>::default(), |acc, route_ctx| {
            route_ctx.state().get_reload_intervals().iter().flat_map(|intervals| intervals.iter()).fold(
                acc,
                |mut acc, &(start_idx, end_idx)| {
                    // get total resource demand for given interval
                    let activity = get_activity_by_idx(route_ctx.route(), start_idx);
                    let resource_demand_with_id = (self.resource_capacity_fn)(activity)
                        .map(|(_, resource_id)| (self.get_total_demand(route_ctx, start_idx..=end_idx), resource_id));

                    if let Some((resource_demand, id)) = resource_demand_with_id {
                        let entry = acc.entry(id).or_default();
                        *entry = *entry + resource_demand;
                    }

                    acc
                },
            )
        });

        // second pass: store amount of available resources inside activity state
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let mut available_resources = vec![None; route_ctx.route().tour.total()];
            let reload_intervals = route_ctx.state().get_reload_intervals().cloned().unwrap_or_default();

            for (start_idx, _) in reload_intervals {
                let activity_idx = get_activity_by_idx(route_ctx.route(), start_idx);
                let resource_available =
                    (self.resource_capacity_fn)(activity_idx).and_then(|(total_capacity, resource_id)| {
                        total_demand.get(&resource_id).map(|total_demand| total_capacity - *total_demand)
                    });

                if let Some(resource_available) = resource_available {
                    available_resources[start_idx] = Some(resource_available);
                }
            }

            route_ctx.state_mut().set_activity_states::<SharedResourceStateKey, Option<T>>(available_resources)
        });
    }

    /// Prevents resource consumption in given route by setting available to zero (default).
    fn prevent_resource_consumption(&self, route_ctx: &mut RouteContext) {
        let mut empty_resources = vec![None; route_ctx.route().tour.total()];

        route_ctx.state().get_reload_intervals().cloned().unwrap_or_default().into_iter().for_each(
            |(start_idx, end_idx)| {
                let activity = get_activity_by_idx(route_ctx.route(), start_idx);
                let has_resource_demand = (self.resource_capacity_fn)(activity).map_or(false, |(_, _)| {
                    (start_idx..=end_idx)
                        .filter_map(|idx| route_ctx.route().tour.get(idx))
                        .filter_map(|activity| activity.job.as_ref())
                        .any(|job| (self.resource_demand_fn)(job).is_some())
                });

                if has_resource_demand {
                    empty_resources[start_idx] = Some(T::default());
                }
            },
        );

        route_ctx.state_mut().set_activity_states::<SharedResourceStateKey, _>(empty_resources)
    }

    fn get_total_demand(&self, route_ctx: &RouteContext, range: RangeInclusive<usize>) -> T {
        range
            .into_iter()
            .filter_map(|idx| route_ctx.route().tour.get(idx))
            .filter_map(|activity| activity.job.as_ref())
            .fold(T::default(), |acc, job| acc + (self.resource_demand_fn)(job).unwrap_or_default())
    }
}

impl<T: SharedResource + Add<Output = T> + Sub<Output = T>> FeatureState for SharedResourceState<T> {
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
}

fn get_activity_by_idx(route: &Route, idx: usize) -> &Activity {
    route.tour.get(idx).expect("cannot get activity by idx")
}

/// Implement `SharedResource` for multi dimensional load.
impl SharedResource for MultiDimLoad {}

/// Implement `SharedResource` for a single dimensional load.
impl SharedResource for SingleDimLoad {}
