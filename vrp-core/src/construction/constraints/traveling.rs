#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/traveling_test.rs"]
mod traveling_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::{Distance, Duration, Profile, Timestamp};
use crate::models::problem::{Actor, Job, TransportCost};
use crate::models::solution::TourActivity;
use std::slice::Iter;
use std::sync::Arc;

pub type TravelLimitFunc = Arc<dyn Fn(&Actor) -> (Option<Distance>, Option<Duration>) + Send + Sync>;

/// A module which allows to limit actor's traveling distance and time.
/// NOTE should be used after Timing module.
pub struct TravelModule {
    limit_func: TravelLimitFunc,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl TravelModule {
    pub fn new(
        limit_func: TravelLimitFunc,
        transport: Arc<dyn TransportCost + Send + Sync>,
        distance_code: i32,
        duration_code: i32,
    ) -> Self {
        Self {
            state_keys: vec![MAX_DISTANCE_KEY, MAX_DURATION_KEY],
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(TravelHardActivityConstraint::new(
                limit_func.clone(),
                distance_code,
                duration_code,
                transport.clone(),
            )))],
            transport,
            limit_func,
        }
    }
}

impl ConstraintModule for TravelModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, _job: &Job) {
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let limit = (self.limit_func)(&ctx.route.actor);

        if limit.0.is_some() || limit.1.is_some() {
            let start = ctx.route.tour.start().unwrap();
            let init = (start.place.location, start.schedule.departure, Distance::default(), Duration::default());

            let (_, _, total_dist, total_dur) =
                ctx.route.tour.all_activities().fold(init, |(loc, dep, total_dist, total_dur), a| {
                    let total_dist = total_dist
                        + self.transport.distance(ctx.route.actor.vehicle.profile, loc, a.place.location, dep);
                    let total_dur = total_dur + a.schedule.departure - dep;

                    (a.place.location, a.schedule.departure, total_dist, total_dur)
                });

            ctx.state_mut().put_route_state(MAX_DISTANCE_KEY, total_dist);
            ctx.state_mut().put_route_state(MAX_DURATION_KEY, total_dur);
        }
    }

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct TravelHardActivityConstraint {
    limit_func: TravelLimitFunc,
    distance_code: i32,
    duration_code: i32,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl HardActivityConstraint for TravelHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let limit = (self.limit_func)(&route_ctx.route.actor);
        if limit.0.is_some() || limit.1.is_some() {
            let (total_distance, total_duration) = self.calculate_travel(route_ctx, activity_ctx);
            match limit {
                (Some(max_distance), _) if max_distance < total_distance => self.violate(self.distance_code),
                (_, Some(max_duration)) if max_duration < total_duration => self.violate(self.duration_code),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl TravelHardActivityConstraint {
    fn new(
        limit_func: TravelLimitFunc,
        distance_code: i32,
        duration_code: i32,
        transport: Arc<dyn TransportCost + Send + Sync>,
    ) -> Self {
        Self { limit_func, distance_code, duration_code, transport }
    }

    fn calculate_travel(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        let actor = &route_ctx.route.actor;
        let profile = actor.vehicle.profile;
        let prev = activity_ctx.prev;
        let tar = activity_ctx.target;
        let next = activity_ctx.next;

        let curr_dis = route_ctx.state.get_route_state(MAX_DISTANCE_KEY).cloned().unwrap_or(0.);
        let curr_dur = route_ctx.state.get_route_state(MAX_DURATION_KEY).cloned().unwrap_or(0.);

        assert_eq!(curr_dur, prev.schedule.departure);

        let (prev_to_tar_dis, prev_to_tar_dur) = self.calculate_leg_travel_info(profile, prev, tar, curr_dur);
        if next.is_none() {
            return (curr_dis + prev_to_tar_dis, curr_dur + prev_to_tar_dur);
        }

        let next = next.unwrap();
        let tar_dep = curr_dur + prev_to_tar_dur;

        let (tar_to_next_dis, tar_to_next_dur) = self.calculate_leg_travel_info(profile, tar, next, tar_dep);

        (curr_dis + prev_to_tar_dis + tar_to_next_dis, curr_dur + prev_to_tar_dur + tar_to_next_dur)
    }

    fn calculate_leg_travel_info(
        &self,
        profile: Profile,
        first: &TourActivity,
        second: &TourActivity,
        departure: Timestamp,
    ) -> (Distance, Duration) {
        let first_to_second_dis =
            self.transport.distance(profile, first.place.location, second.place.location, departure);
        let first_to_second_dur =
            self.transport.duration(profile, first.place.location, second.place.location, departure);

        let second_arr = departure + first_to_second_dur;
        let second_wait = (second.place.time.start - (departure + first_to_second_dur)).max(0.);
        let second_dep = second_arr + second_wait + second.place.duration;

        (first_to_second_dis, second_dep - departure)
    }

    fn violate(&self, code: i32) -> Option<ActivityConstraintViolation> {
        Some(ActivityConstraintViolation { code, stopped: false })
    }
}
