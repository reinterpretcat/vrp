#[cfg(test)]
#[path = "../../tests/unit/constraints/reloads_test.rs"]
mod reloads_test;

use crate::constraints::*;
use hashbrown::{HashMap, HashSet};
use std::marker::PhantomData;
use std::ops::{Deref, Range};
use vrp_core::construction::constraints::*;
use vrp_core::construction::extensions::MultiTrip;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::*;
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::Route;

/// Creates a multi trip strategy to use multi trip with reload jobs.
pub fn create_reload_multi_trip<T: LoadOps>(
    load_schedule_threshold_fn: Box<dyn Fn(&T) -> T + Send + Sync>,
    place_capacity_threshold: Option<Box<dyn Fn(&Activity, &T) -> bool + Send + Sync>>,
) -> impl MultiTrip<Constraint = T> + Send + Sync {
    FixedMultiTrip {
        is_marker_single: Box::new(is_reload_single),
        is_multi_trip_needed: Box::new(move |route_ctx| {
            route_ctx
                .route
                .tour
                .end()
                .map(|end| {
                    let current: T =
                        route_ctx.state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).cloned().unwrap_or_default();
                    let max_capacity = route_ctx.route.actor.vehicle.dimens.get_capacity().unwrap();

                    current >= load_schedule_threshold_fn.deref()(max_capacity)
                })
                .unwrap_or(false)
        }),
        is_obsolete_interval: Box::new(move |route_ctx, left, right| {
            let capacity: T = route_ctx.route.actor.vehicle.dimens.get_capacity().cloned().unwrap_or_default();

            let get_load = |activity_index: usize, state_key: i32| {
                let activity = route_ctx.route.tour.get(activity_index).unwrap();
                route_ctx.state.get_activity_state::<T>(state_key, activity).cloned().unwrap_or_default()
            };

            let fold_demand = |range: Range<usize>, demand_fn: fn(&Demand<T>) -> T| {
                route_ctx.route.tour.activities_slice(range.start, range.end).iter().fold(
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

            let has_enough_vehicle_capacity = capacity >= new_max_load_left && capacity >= new_max_load_right;

            has_enough_vehicle_capacity
                && place_capacity_threshold.as_ref().map_or(true, |place_capacity_threshold| {
                    // total static delivery at left
                    let left_delivery = fold_demand(left.start..right.end, |demand| demand.delivery.0);
                    let activity = route_ctx.route.tour.get(left.start).unwrap();

                    place_capacity_threshold.deref()(activity, &left_delivery)
                })
        }),
        intervals_key: RELOAD_INTERVALS_KEY,
        phantom: Default::default(),
    }
}

/// Creates a shared resource constraint module to constraint reload jobs.
pub fn create_shared_reload_constraint<T>(
    resource_map: HashMap<Job, (T, SharedResourceId)>,
    total_jobs: usize,
    constraint_code: i32,
    resource_key: i32,
) -> SharedResourceModule<T>
where
    T: SharedResource + LoadOps,
{
    SharedResourceModule::new(
        total_jobs,
        constraint_code,
        resource_key,
        Arc::new(move |route_ctx| route_ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY)),
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

/// Specifies obsolete interval function which takes left and right interval range. These
/// intervals are separated by marker job activity.
type ObsoleteIntervalFn = dyn Fn(&RouteContext, Range<usize>, Range<usize>) -> bool + Send + Sync;

struct FixedMultiTrip<T: Send + Sync> {
    is_marker_single: Box<dyn Fn(&Single) -> bool + Send + Sync>,
    is_multi_trip_needed: Box<dyn Fn(&RouteContext) -> bool + Send + Sync>,
    is_obsolete_interval: Box<ObsoleteIntervalFn>,
    intervals_key: i32,
    phantom: PhantomData<T>,
}

impl<T: Send + Sync> MultiTrip for FixedMultiTrip<T> {
    type Constraint = T;

    fn is_marker_job(&self, job: &Job) -> bool {
        job.as_single().map_or(false, |single| self.is_marker_single.deref()(single))
    }

    fn is_assignable(&self, route: &Route, job: &Job) -> bool {
        if self.is_marker_job(job) {
            let job = job.to_single();
            let vehicle_id = get_vehicle_id_from_job(job);
            let shift_index = get_shift_index(&job.dimens);

            is_correct_vehicle(route, vehicle_id, shift_index)
        } else {
            false
        }
    }

    fn is_multi_trip_needed(&self, route_ctx: &RouteContext) -> bool {
        self.is_multi_trip_needed.deref()(route_ctx)
    }

    fn get_state_code(&self) -> Option<i32> {
        Some(self.intervals_key)
    }

    fn filter_markers<'a>(
        &'a self,
        route: &'a Route,
        jobs: &'a [Job],
    ) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync> {
        let shift_index = get_shift_index(&route.actor.vehicle.dimens);
        let vehicle_id = route.actor.vehicle.dimens.get_id().unwrap();

        Box::new(
            jobs.iter()
                .filter(move |job| match job {
                    Job::Single(job) => {
                        self.is_marker_single.deref()(job)
                            && get_shift_index(&job.dimens) == shift_index
                            && get_vehicle_id_from_job(job) == vehicle_id
                    }
                    _ => false,
                })
                .cloned(),
        )
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        self.promote_multi_trips_when_needed(solution_ctx);
        self.remove_trivial_multi_trips(solution_ctx);
    }
}

impl<T: Send + Sync> FixedMultiTrip<T> {
    fn remove_trivial_multi_trips(&self, solution_ctx: &mut SolutionContext) {
        let mut extra_ignored = Vec::new();
        solution_ctx.routes.iter_mut().filter(|route_ctx| self.has_markers(route_ctx)).for_each(|route_ctx| {
            let reloads = self.get_marker_intervals(route_ctx).cloned().unwrap_or_default();

            let _ = reloads.windows(2).try_for_each(|item| {
                let ((left_start, left_end), (right_start, right_end)) = match item {
                    &[left, right] => (left, right),
                    _ => unreachable!(),
                };

                assert_eq!(left_end + 1, right_start);

                if self.is_obsolete_interval.deref()(route_ctx, left_start..left_end, right_start..right_end) {
                    // NOTE: we remove only one reload per tour, state update should be handled externally
                    extra_ignored.push(route_ctx.route_mut().tour.remove_activity_at(right_start));
                    Err(())
                } else {
                    Ok(())
                }
            });
        });

        solution_ctx.ignored.extend(extra_ignored.into_iter());
    }

    fn promote_multi_trips_when_needed(&self, solution_ctx: &mut SolutionContext) {
        let jobs = solution_ctx
            .routes
            .iter()
            .filter(|route_ctx| self.is_multi_trip_needed(route_ctx))
            .flat_map(|route_ctx| {
                self.filter_markers(&route_ctx.route, &solution_ctx.ignored)
                    .chain(self.filter_markers(&route_ctx.route, &solution_ctx.required))
            })
            .collect::<HashSet<_>>();

        solution_ctx.ignored.retain(|job| !jobs.contains(job));
        solution_ctx.locked.extend(jobs.iter().cloned());
        solution_ctx.required.extend(jobs.into_iter());
    }
}

fn is_reload_single(single: &Single) -> bool {
    single.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}
