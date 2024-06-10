//! A reloads feature.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/reloads_test.rs"]
mod reloads_test;

use super::*;
use crate::construction::enablers::{FeatureCombinator, RouteIntervals};
use crate::models::problem::Single;
use crate::models::solution::Route;
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::ops::Range;

/// Specifies dependencies needed to use reload feature.
pub trait ReloadAspects<T: LoadOps>: Clone + Send + Sync {
    /// Checks whether the job is a reload job and it can be assigned to the given route.
    fn belongs_to_route(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether the single job is reload job.
    fn is_reload_single(&self, single: &Single) -> bool;

    /// Returns capacity of the vehicle.
    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T>;

    /// Returns demand of the single job if it is specified.
    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>>;
}

/// Specifies load schedule threshold function.
pub type LoadScheduleThresholdFn<T> = Box<dyn Fn(&T) -> T + Send + Sync>;
/// A factory function to create capacity feature.
pub type CapacityFeatureFactoryFn = Box<dyn FnOnce(&str, RouteIntervals) -> Result<Feature, GenericError>>;
/// Specifies place capacity threshold function.
type PlaceCapacityThresholdFn<T> = Box<dyn Fn(&RouteContext, usize, &T) -> bool + Send + Sync>;

/// Keys to track state of reload feature.
#[derive(Clone, Debug)]
pub struct ReloadKeys {
    /// Reload intervals key.
    pub intervals: StateKey,
    /// Capacity feature keys.
    pub capacity_keys: CapacityKeys,
}

/// Keys to track state of reload feature.
#[derive(Clone, Debug)]
pub struct SharedReloadKeys {
    /// Shared resource key.
    pub resource: StateKey,
    /// Reload keys.
    pub reload_keys: ReloadKeys,
}

/// Creates a multi trip strategy to use multi trip with reload jobs which shared some resources.
#[allow(clippy::too_many_arguments)]
pub fn create_shared_reload_multi_trip_feature<T, A>(
    name: &str,
    capacity_feature_factory: CapacityFeatureFactoryFn,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    resource_map: HashMap<Job, (T, SharedResourceId)>,
    total_jobs: usize,
    shared_reload_keys: SharedReloadKeys,
    violation_code: ViolationCode,
    aspects: A,
) -> Result<Feature, GenericError>
where
    T: SharedResource + LoadOps,
    A: ReloadAspects<T> + 'static,
{
    let shared_resource = create_shared_reload_constraint(
        name,
        resource_map,
        total_jobs,
        shared_reload_keys.clone(),
        violation_code,
        aspects.clone(),
    )?;

    let route_intervals = create_reload_route_intervals(
        shared_reload_keys.reload_keys.clone(),
        load_schedule_threshold_fn,
        Some(Box::new(move |route_ctx, activity_idx, demand| {
            route_ctx
                .state()
                .get_activity_state::<T>(shared_reload_keys.resource, activity_idx)
                .map_or(true, |resource_available| resource_available.can_fit(demand))
        })),
        aspects,
    );
    let capacity = (capacity_feature_factory)(name, route_intervals)?;

    FeatureCombinator::default().use_name(name).add_features(&[capacity, shared_resource]).combine()
}

/// Creates a multi trip feature to use multi trip with reload jobs.
pub fn create_simple_reload_multi_trip_feature<T, A>(
    name: &str,
    capacity_feature_factory: CapacityFeatureFactoryFn,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    reload_keys: ReloadKeys,
    aspects: A,
) -> Result<Feature, GenericError>
where
    T: SharedResource + LoadOps,
    A: ReloadAspects<T> + 'static,
{
    (capacity_feature_factory)(
        name,
        create_simple_reload_route_intervals(load_schedule_threshold_fn, reload_keys, aspects),
    )
}

/// Creates a reload intervals to use with reload jobs.
pub fn create_simple_reload_route_intervals<T, A>(
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    reload_keys: ReloadKeys,
    aspects: A,
) -> RouteIntervals
where
    T: SharedResource + LoadOps,
    A: ReloadAspects<T> + 'static,
{
    create_reload_route_intervals(reload_keys, load_schedule_threshold_fn, None, aspects)
}

fn create_reload_route_intervals<T, A>(
    reload_keys: ReloadKeys,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    place_capacity_threshold: Option<PlaceCapacityThresholdFn<T>>,
    aspects: A,
) -> RouteIntervals
where
    T: SharedResource + LoadOps,
    A: ReloadAspects<T> + 'static,
{
    let capacity_keys = reload_keys.capacity_keys;
    RouteIntervals::Multiple {
        is_marker_single_fn: {
            let aspects = aspects.clone();
            Arc::new(move |single| aspects.is_reload_single(single))
        },
        is_new_interval_needed_fn: {
            let aspects = aspects.clone();
            Arc::new(move |route_ctx| {
                route_ctx
                    .route()
                    .tour
                    .end_idx()
                    .map(|end_idx| {
                        let current: T = route_ctx
                            .state()
                            .get_activity_state(capacity_keys.max_past_capacity, end_idx)
                            .cloned()
                            .unwrap_or_default();

                        let max_capacity =
                            aspects.get_capacity(&route_ctx.route().actor.vehicle).cloned().unwrap_or_default();
                        let threshold_capacity = (load_schedule_threshold_fn)(&max_capacity);

                        current.partial_cmp(&threshold_capacity) != Some(Ordering::Less)
                    })
                    .unwrap_or(false)
            })
        },
        is_obsolete_interval_fn: {
            let aspects = aspects.clone();
            Arc::new(move |route_ctx, left, right| {
                let capacity: T = aspects.get_capacity(&route_ctx.route().actor.vehicle).cloned().unwrap_or_default();

                let get_load = |activity_idx: usize, state_key: StateKey| {
                    route_ctx.state().get_activity_state::<T>(state_key, activity_idx).cloned().unwrap_or_default()
                };

                let fold_demand = |range: Range<usize>, demand_fn: fn(&Demand<T>) -> T| {
                    route_ctx.route().tour.activities_slice(range.start, range.end).iter().fold(
                        T::default(),
                        |acc, activity| {
                            activity
                                .job
                                .as_ref()
                                .and_then(|job| aspects.get_demand(job))
                                .map(|demand| acc + demand_fn(demand))
                                .unwrap_or_else(|| acc)
                        },
                    )
                };

                let left_pickup = fold_demand(left.clone(), |demand| demand.pickup.0);
                let right_delivery = fold_demand(right.clone(), |demand| demand.delivery.0);

                // static delivery moved to left
                let new_max_load_left = get_load(left.start, capacity_keys.max_future_capacity) + right_delivery;
                // static pickup moved to right
                let new_max_load_right = get_load(right.start, capacity_keys.max_future_capacity) + left_pickup;

                let has_enough_vehicle_capacity =
                    capacity.can_fit(&new_max_load_left) && capacity.can_fit(&new_max_load_right);

                has_enough_vehicle_capacity
                    && place_capacity_threshold.as_ref().map_or(true, |place_capacity_threshold| {
                        // total static delivery at left
                        let left_delivery = fold_demand(left.start..right.end, |demand| demand.delivery.0);

                        (place_capacity_threshold)(route_ctx, left.start, &left_delivery)
                    })
            })
        },
        is_assignable_fn: Arc::new(move |route, job| aspects.belongs_to_route(route, job)),
        intervals_key: reload_keys.intervals,
    }
}

/// Creates a shared resource constraint module to constraint reload jobs.
fn create_shared_reload_constraint<T, A>(
    name: &str,
    resource_map: HashMap<Job, (T, SharedResourceId)>,
    total_jobs: usize,
    shared_reload_keys: SharedReloadKeys,
    constraint_code: ViolationCode,
    aspects: A,
) -> Result<Feature, GenericError>
where
    T: SharedResource + LoadOps,
    A: ReloadAspects<T> + 'static,
{
    let intervals_key = shared_reload_keys.reload_keys.intervals;
    create_shared_resource_feature(
        name,
        total_jobs,
        constraint_code,
        shared_reload_keys.resource,
        Arc::new(move |route_ctx| route_ctx.state().get_route_state::<Vec<(usize, usize)>>(intervals_key)),
        Arc::new(move |activity| {
            activity.job.as_ref().and_then(|job| {
                if aspects.is_reload_single(job.as_ref()) {
                    resource_map.get(&Job::Single(job.clone())).cloned()
                } else {
                    None
                }
            })
        }),
        Arc::new(|single| single.dimens.get_demand().map(|demand| demand.delivery.0)),
    )
}
