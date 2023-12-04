//! A reloads feature.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/reloads_test.rs"]
mod reloads_test;

use super::*;
use crate::construction::enablers::JobTie;
use crate::construction::enablers::*;
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::ops::Range;
use vrp_core::construction::enablers::{FixedRouteIntervals, RouteIntervals};
use vrp_core::construction::features::*;
use vrp_core::models::problem::Single;
use vrp_core::models::solution::Activity;

/// Specifies load schedule threshold function.
pub type LoadScheduleThresholdFn<T> = Box<dyn Fn(&T) -> T + Send + Sync>;
/// A factory function to create capacity feature.
pub type CapacityFeatureFactoryFn =
    Box<dyn Fn(&str, Arc<dyn RouteIntervals + Send + Sync>) -> Result<Feature, GenericError>>;
/// Specifies place capacity threshold function.
type PlaceCapacityThresholdFn<T> = Box<dyn Fn(&RouteContext, &Activity, &T) -> bool + Send + Sync>;

/// Creates a multi trip strategy to use multi trip with reload jobs which shared some resources.
pub fn create_shared_reload_multi_trip_feature<T>(
    name: &str,
    capacity_feature_factory: CapacityFeatureFactoryFn,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    resource_map: HashMap<Job, (T, SharedResourceId)>,
    total_jobs: usize,
    constraint_code: ViolationCode,
    resource_key: StateKey,
) -> Result<Feature, GenericError>
where
    T: SharedResource + LoadOps,
{
    let shared_resource =
        create_shared_reload_constraint(name, resource_map, total_jobs, constraint_code, resource_key)?;

    let route_intervals = create_reload_route_intervals(
        load_schedule_threshold_fn,
        Some(Box::new(move |route_ctx, activity, demand| {
            route_ctx
                .state()
                .get_activity_state::<T>(resource_key, activity)
                .map_or(true, |resource_available| resource_available.can_fit(demand))
        })),
    );
    let capacity = (capacity_feature_factory)(name, Arc::new(route_intervals))?;

    FeatureBuilder::combine(name, &[capacity, shared_resource])
}

/// Creates a multi trip feature to use multi trip with reload jobs.
pub fn create_simple_reload_multi_trip_feature<T: LoadOps>(
    name: &str,
    capacity_feature_factory: CapacityFeatureFactoryFn,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
) -> Result<Feature, GenericError> {
    (capacity_feature_factory)(name, create_simple_reload_route_intervals(load_schedule_threshold_fn))
}

/// Creates a reload intervals to use with reload jobs.
pub fn create_simple_reload_route_intervals<T: LoadOps>(
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
) -> Arc<dyn RouteIntervals + Send + Sync> {
    Arc::new(create_reload_route_intervals(load_schedule_threshold_fn, None))
}

fn create_reload_route_intervals<T: LoadOps>(
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
    place_capacity_threshold: Option<PlaceCapacityThresholdFn<T>>,
) -> FixedRouteIntervals {
    FixedRouteIntervals {
        is_marker_single_fn: Box::new(is_reload_single),
        is_new_interval_needed_fn: Box::new(move |route_ctx| {
            route_ctx
                .route()
                .tour
                .end()
                .map(|end| {
                    let current: T =
                        route_ctx.state().get_activity_state(MAX_PAST_CAPACITY_KEY, end).cloned().unwrap_or_default();

                    let max_capacity = route_ctx.route().actor.vehicle.dimens.get_capacity().unwrap();
                    let threshold_capacity = (load_schedule_threshold_fn)(max_capacity);

                    current.partial_cmp(&threshold_capacity) != Some(Ordering::Less)
                })
                .unwrap_or(false)
        }),
        is_obsolete_interval_fn: Box::new(move |route_ctx, left, right| {
            let capacity: T = route_ctx.route().actor.vehicle.dimens.get_capacity().cloned().unwrap_or_default();

            let get_load = |activity_index: usize, state_key: i32| {
                let activity = route_ctx.route().tour.get(activity_index).unwrap();
                route_ctx.state().get_activity_state::<T>(state_key, activity).cloned().unwrap_or_default()
            };

            let fold_demand = |range: Range<usize>, demand_fn: fn(&Demand<T>) -> T| {
                route_ctx.route().tour.activities_slice(range.start, range.end).iter().fold(
                    T::default(),
                    |acc, activity| {
                        activity
                            .job
                            .as_ref()
                            .and_then(|job| job.dimens.get_demand())
                            .map(|demand| acc + demand_fn(demand))
                            .unwrap_or_else(|| acc)
                    },
                )
            };

            let left_pickup = fold_demand(left.clone(), |demand| demand.pickup.0);
            let right_delivery = fold_demand(right.clone(), |demand| demand.delivery.0);

            // static delivery moved to left
            let new_max_load_left = get_load(left.start, MAX_FUTURE_CAPACITY_KEY) + right_delivery;
            // static pickup moved to right
            let new_max_load_right = get_load(right.start, MAX_FUTURE_CAPACITY_KEY) + left_pickup;

            let has_enough_vehicle_capacity =
                capacity.can_fit(&new_max_load_left) && capacity.can_fit(&new_max_load_right);

            has_enough_vehicle_capacity
                && place_capacity_threshold.as_ref().map_or(true, |place_capacity_threshold| {
                    // total static delivery at left
                    let left_delivery = fold_demand(left.start..right.end, |demand| demand.delivery.0);
                    let activity = route_ctx.route().tour.get(left.start).unwrap();

                    (place_capacity_threshold)(route_ctx, activity, &left_delivery)
                })
        }),
        is_assignable_fn: Box::new(|route, job| {
            job.as_single().map_or(false, |job| {
                is_correct_vehicle(route, get_vehicle_id_from_job(job), get_shift_index(&job.dimens))
            })
        }),
        intervals_key: RELOAD_INTERVALS_KEY,
    }
}

/// Creates a shared resource constraint module to constraint reload jobs.
fn create_shared_reload_constraint<T>(
    name: &str,
    resource_map: HashMap<Job, (T, SharedResourceId)>,
    total_jobs: usize,
    constraint_code: ViolationCode,
    resource_key: StateKey,
) -> Result<Feature, GenericError>
where
    T: SharedResource + LoadOps,
{
    create_shared_resource_feature(
        name,
        total_jobs,
        constraint_code,
        resource_key,
        Arc::new(move |route_ctx| route_ctx.state().get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY)),
        Arc::new(move |activity| {
            activity.job.as_ref().and_then(|job| {
                if is_reload_single(job.as_ref()) {
                    resource_map.get(&Job::Single(job.clone())).cloned()
                } else {
                    None
                }
            })
        }),
        Arc::new(|single| single.dimens.get_demand().map(|demand| demand.delivery.0)),
    )
}

fn is_reload_single(single: &Single) -> bool {
    single.dimens.get_job_type().map_or(false, |t| t == "reload")
}
