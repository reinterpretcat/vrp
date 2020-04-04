use crate::constraints::*;
use std::ops::{Add, Deref, Sub};
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::heuristics::RouteContext;
use vrp_core::models::common::{IdDimension, ValueDimension};
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::{Activity, Route};

/// A strategy to use multi trip with reload jobs.
pub struct ReloadMultiTrip<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    ReloadMultiTrip<Capacity>
{
    pub fn new(threshold: Box<dyn Fn(&Capacity) -> Capacity + Send + Sync>) -> Self {
        Self { threshold }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    MultiTrip<Capacity> for ReloadMultiTrip<Capacity>
{
    fn is_reload_job(&self, job: &Job) -> bool {
        job.as_single().map_or(false, |single| self.is_reload_single(single))
    }

    fn is_reload_single(&self, single: &Single) -> bool {
        single.dimens.get_value::<String>("type").map_or(false, |t| t == "reload")
    }

    fn is_assignable(&self, route: &Route, job: &Job) -> bool {
        if self.is_reload_job(job) {
            let job = job.to_single();
            let vehicle_id = get_vehicle_id_from_job(&job).unwrap();
            let shift_index = get_shift_index(&job.dimens);

            is_correct_vehicle(route, vehicle_id, shift_index)
        } else {
            false
        }
    }

    fn is_reload_needed(&self, current: &Capacity, max_capacity: &Capacity) -> bool {
        *current >= self.threshold.deref()(max_capacity)
    }

    fn has_reloads(&self, route_ctx: &RouteContext) -> bool {
        route_ctx
            .state
            .get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS)
            .map(|intervals| intervals.len() > 1)
            .unwrap_or(false)
    }

    fn get_reload<'a>(&self, activity: &'a Activity) -> Option<&'a Arc<Single>> {
        as_single_job(activity, |job| self.is_reload_single(job))
    }

    fn get_reloads<'a>(
        &'a self,
        route: &'a Route,
        jobs: &'a Vec<Job>,
    ) -> Box<dyn Iterator<Item = Job> + 'a + Send + Sync> {
        let shift_index = get_shift_index(&route.actor.vehicle.dimens);
        let vehicle_id = route.actor.vehicle.dimens.get_id().unwrap();

        Box::new(
            jobs.iter()
                .filter(move |job| match job {
                    Job::Single(job) => {
                        self.is_reload_single(&job)
                            && get_shift_index(&job.dimens) == shift_index
                            && get_vehicle_id_from_job(&job).unwrap() == vehicle_id
                    }
                    _ => false,
                })
                .cloned(),
        )
    }
}
