#[cfg(test)]
#[path = "../../tests/unit/constraints/reload_test.rs"]
mod reload_test;

use crate::constraints::*;
use std::ops::Deref;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::{RouteContext, SolutionContext};
use vrp_core::models::common::{CapacityDimension, Demand, DemandDimension, IdDimension, LoadOps, ValueDimension};
use vrp_core::models::problem::{ActivityCost, Job, Single, TransportCost};
use vrp_core::models::solution::{Activity, Route};

/// A strategy to use multi trip with reload jobs.
pub struct ReloadMultiTrip<T: LoadOps> {
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    threshold: Box<dyn Fn(&T) -> T + Send + Sync>,
}

impl<T: LoadOps> ReloadMultiTrip<T> {
    pub fn new(
        activity: Arc<dyn ActivityCost + Send + Sync>,
        transport: Arc<dyn TransportCost + Send + Sync>,
        threshold: Box<dyn Fn(&T) -> T + Send + Sync>,
    ) -> Self {
        Self { activity, transport, threshold }
    }
}

impl<T: LoadOps> MultiTrip for ReloadMultiTrip<T> {
    type Capacity = T;

    fn is_reload_job(&self, job: &Job) -> bool {
        job.as_single().map_or(false, |single| self.is_reload_single(single))
    }

    fn is_reload_single(&self, single: &Single) -> bool {
        single.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
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
                let current: Self::Capacity =
                    ctx.state.get_activity_state(MAX_PAST_CAPACITY_KEY, end).cloned().unwrap_or_default();
                let max_capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();

                current >= self.threshold.deref()(max_capacity)
            })
            .unwrap_or(false)
    }

    fn has_reloads(&self, route_ctx: &RouteContext) -> bool {
        route_ctx
            .state
            .get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS_KEY)
            .map(|intervals| intervals.len() > 1)
            .unwrap_or(false)
    }

    fn get_reload<'a>(&self, activity: &'a Activity) -> Option<&'a Arc<Single>> {
        as_single_job(activity, |job| self.is_reload_single(job))
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
                        self.is_reload_single(job)
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
        // removes reloads at the start and end of tour
        let mut extra_ignored = Vec::new();
        ctx.routes.iter_mut().filter(|ctx| self.has_reloads(ctx)).for_each(|rc| {
            let demands = (0..)
                .zip(rc.route.tour.all_activities())
                .filter_map(|(idx, activity)| get_demand::<Self::Capacity>(activity).map(|_| idx))
                .collect::<Vec<_>>();

            let (start, end) =
                (demands.first().cloned().unwrap_or(0), demands.last().cloned().unwrap_or(rc.route.tour.total() - 1));

            (0..)
                .zip(rc.route.tour.all_activities())
                .filter_map(|(idx, activity)| self.get_reload(activity).map(|reload| (reload.clone(), idx)))
                .filter(|(_, idx)| *idx < start || *idx > end)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .for_each(|(reload, _)| {
                    let job = Job::Single(reload);
                    assert!(rc.route_mut().tour.remove(&job));
                    extra_ignored.push(job);
                });

            if rc.is_stale() {
                self.accept_route_state(rc);
                update_route_schedule(rc, self.activity.as_ref(), self.transport.as_ref());
            }
        });
        ctx.ignored.extend(extra_ignored.into_iter());
    }
}

fn get_demand<T: LoadOps>(activity: &Activity) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| job.dimens.get_demand())
}
