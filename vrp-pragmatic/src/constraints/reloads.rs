#[cfg(test)]
#[path = "../../tests/unit/constraints/reload_test.rs"]
mod reload_test;

use crate::constraints::*;
use std::ops::Deref;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::*;
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::{Activity, Route};

/// A strategy to use multi trip with reload jobs.
pub struct ReloadMultiTrip<T: LoadOps> {
    threshold: Box<dyn Fn(&T) -> T + Send + Sync>,
}

impl<T: LoadOps> ReloadMultiTrip<T> {
    pub fn new(threshold: Box<dyn Fn(&T) -> T + Send + Sync>) -> Self {
        Self { threshold }
    }
}

impl<T: LoadOps> MultiTrip for ReloadMultiTrip<T> {
    type Capacity = T;

    fn is_reload_job(&self, job: &Job) -> bool {
        job.as_single().map_or(false, |single| is_reload_single(single))
    }

    fn is_assignable(&self, route: &Route, job: &Job) -> bool {
        if self.is_reload_job(job) {
            let job = job.to_single();
            let vehicle_id = get_vehicle_id_from_job(job).unwrap();
            let shift_index = get_shift_index(&job.dimens);

            is_correct_vehicle(route, vehicle_id, shift_index)
        } else {
            false
        }
    }

    fn is_vehicle_full(&self, ctx: &RouteContext) -> bool {
        ctx.route
            .tour
            .end()
            .map(|end| {
                let current: T = ctx.state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).cloned().unwrap_or_default();
                let max_capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();

                current >= self.threshold.deref()(max_capacity)
            })
            .unwrap_or(false)
    }

    fn has_reloads(&self, route_ctx: &RouteContext) -> bool {
        get_reloads(route_ctx).map(|intervals| intervals.len() > 1).unwrap_or(false)
    }

    fn get_reload<'a>(&self, activity: &'a Activity) -> Option<&'a Arc<Single>> {
        as_single_job(activity, |job| is_reload_single(job))
    }

    fn get_all_reloads<'a>(
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
                        is_reload_single(job)
                            && get_shift_index(&job.dimens) == shift_index
                            && get_vehicle_id_from_job(job).unwrap() == vehicle_id
                    }
                    _ => false,
                })
                .cloned(),
        )
    }

    fn get_reload_intervals(&self, route_ctx: &RouteContext) -> Option<Vec<(usize, usize)>> {
        route_ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY).cloned()
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let (route, state) = route_ctx.as_mut();
        let intervals = route_intervals(route, |a| self.get_reload(a).is_some());
        state.put_route_state(RELOAD_INTERVALS_KEY, intervals);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        let mut extra_ignored = Vec::new();
        ctx.routes.iter_mut().filter(|route_ctx| self.has_reloads(route_ctx)).for_each(|route_ctx| {
            let reloads = get_reloads(route_ctx).cloned().unwrap_or_default();
            let capacity: T = route_ctx.route.actor.vehicle.dimens.get_capacity().cloned().unwrap_or_default();

            let _ = reloads.windows(2).try_for_each(|item| {
                let ((left_start, left_end), (right_start, right_end)) = match item {
                    &[left, right] => (left, right),
                    _ => unreachable!(),
                };

                assert_eq!(left_end + 1, right_start);

                let is_obsolete_reload = {
                    let get_load = |activity_index: usize, state_key: i32| {
                        let activity = route_ctx.route.tour.get(activity_index).unwrap();
                        route_ctx
                            .state
                            .get_activity_state::<Self::Capacity>(state_key, activity)
                            .cloned()
                            .unwrap_or_default()
                    };

                    let fold_demand = |range: (usize, usize), demand_fn: fn(&Demand<T>) -> T| {
                        route_ctx.route.tour.activities_slice(range.0, range.1).iter().fold(
                            T::default(),
                            |acc, activity| {
                                get_demand(activity).map(|demand| acc + demand_fn(demand)).unwrap_or_else(|| acc)
                            },
                        )
                    };

                    let left_pickup = fold_demand((left_start, left_end), |demand| demand.pickup.0);
                    let right_delivery = fold_demand((right_start, right_end), |demand| demand.delivery.0);

                    // static delivery moved to left
                    let new_max_load_left = get_load(left_start, MAX_FUTURE_CAPACITY_KEY) + right_delivery;
                    // static pickup moved to right
                    let new_max_load_right = get_load(right_start, MAX_FUTURE_CAPACITY_KEY) + left_pickup;

                    capacity >= new_max_load_left && capacity >= new_max_load_right
                };

                if is_obsolete_reload {
                    // NOTE: we remove only one reload per tour, state update should be handled externally
                    extra_ignored.push(route_ctx.route_mut().tour.remove_activity_at(right_start));
                    Err(())
                } else {
                    Ok(())
                }
            });
        });

        ctx.ignored.extend(extra_ignored.into_iter());
    }
}

fn get_demand<T: LoadOps>(activity: &Activity) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| job.dimens.get_demand())
}

fn is_reload_single(single: &Single) -> bool {
    single.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
}

fn get_reloads(route_ctx: &RouteContext) -> Option<&Vec<(usize, usize)>> {
    route_ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY)
}
