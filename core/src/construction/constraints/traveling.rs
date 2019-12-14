#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/traveling_test.rs"]
mod traveling_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{Actor, Job, TransportCost};
use std::slice::Iter;
use std::sync::Arc;

pub type TravelLimitFunc = Arc<dyn Fn(&Actor) -> (Option<Distance>, Option<Duration>) + Send + Sync>;

/// Allows to limit actor's traveling distance and time.
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
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, _job: &Arc<Job>) {
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
    duration_func: Box<dyn Fn(Profile, Location, Location, Timestamp) -> f64 + Send + Sync>,
    distance_func: Box<dyn Fn(Profile, Location, Location, Timestamp) -> f64 + Send + Sync>,
}

impl HardActivityConstraint for TravelHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let limit = (self.limit_func)(&route_ctx.route.actor);

        if let Some(max_distance) = limit.0 {
            if let Some(violation) = self.check_distance(route_ctx, activity_ctx, max_distance) {
                return Some(violation);
            }
        }

        if let Some(max_duration) = limit.1 {
            if let Some(violation) = self.check_duration(route_ctx, activity_ctx, max_duration) {
                return Some(violation);
            }
        }

        None
    }
}

impl TravelHardActivityConstraint {
    fn new(
        limit_func: TravelLimitFunc,
        distance_code: i32,
        duration_code: i32,
        transport: Arc<dyn TransportCost + Send + Sync>,
    ) -> Self {
        let transport_copy = transport.clone();
        Self {
            limit_func,
            distance_code,
            duration_code,
            duration_func: Box::new(move |profile, from, to, departure| {
                transport.duration(profile, from, to, departure)
            }),
            distance_func: Box::new(move |profile, from, to, departure| {
                transport_copy.distance(profile, from, to, departure)
            }),
        }
    }

    fn check_distance(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        max_distance: f64,
    ) -> Option<ActivityConstraintViolation> {
        if self.check_travel(route_ctx, activity_ctx, max_distance, MAX_DISTANCE_KEY, &(self.distance_func)) {
            None
        } else {
            Some(ActivityConstraintViolation { code: self.distance_code, stopped: false })
        }
    }

    fn check_duration(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        max_duration: f64,
    ) -> Option<ActivityConstraintViolation> {
        // NOTE consider extra operation time
        let max_duration = max_duration - activity_ctx.target.place.duration;

        if self.check_travel(route_ctx, activity_ctx, max_duration, MAX_DURATION_KEY, &(self.duration_func)) {
            None
        } else {
            Some(ActivityConstraintViolation { code: self.duration_code, stopped: false })
        }
    }

    fn check_travel(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        limit: f64,
        key: i32,
        func: &Box<dyn Fn(Profile, Location, Location, Timestamp) -> f64 + Send + Sync>,
    ) -> bool {
        let actor = &route_ctx.route.actor;
        let current = *route_ctx.state.get_route_state(key).unwrap_or(&0.);

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let prev_to_target =
            func(actor.vehicle.profile, prev.place.location, target.place.location, prev.schedule.departure);

        if next.is_none() {
            return current + prev_to_target <= limit;
        }

        let next = next.unwrap();

        let prev_to_next =
            func(actor.vehicle.profile, prev.place.location, next.place.location, prev.schedule.departure);
        let target_to_next =
            func(actor.vehicle.profile, target.place.location, next.place.location, target.schedule.departure);

        current + prev_to_target + target_to_next - prev_to_next <= limit
    }
}
