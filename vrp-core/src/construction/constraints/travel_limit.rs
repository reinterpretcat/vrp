use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::solution::*;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// A module which controls travel limits.
pub struct TravelLimitModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    tour_duration_limit: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
}

impl ConstraintModule for TravelLimitModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(limit_duration) = self.tour_duration_limit.deref()(route_ctx.route.actor.as_ref()) {
            route_ctx.state_mut().put_route_state(LIMIT_DURATION_KEY, limit_duration);
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}

    fn merge(&self, source: Job, _: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl TravelLimitModule {
    /// Creates a new instance of `TravelLimitModule`.
    pub fn new(
        transport: Arc<dyn TransportCost + Send + Sync>,
        tour_distance_limit: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
        tour_duration_limit: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
        distance_code: i32,
        duration_code: i32,
    ) -> Self {
        Self {
            tour_duration_limit: tour_duration_limit.clone(),
            state_keys: Vec::default(),
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(TravelHardActivityConstraint {
                distance_code,
                duration_code,
                transport,
                tour_distance_limit,
                tour_duration_limit,
            }))],
        }
    }
}

/// A hard activity constraint which allows to limit actor's traveling distance and time.
struct TravelHardActivityConstraint {
    distance_code: i32,
    duration_code: i32,
    transport: Arc<dyn TransportCost + Send + Sync>,
    tour_distance_limit: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
    tour_duration_limit: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
}

impl HardActivityConstraint for TravelHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let tour_distance_limit = self.tour_distance_limit.deref()(route_ctx.route.actor.as_ref());
        let tour_duration_limit = self.tour_duration_limit.deref()(route_ctx.route.actor.as_ref());

        if tour_distance_limit.is_some() || tour_duration_limit.is_some() {
            let (change_distance, change_duration) = self.calculate_travel(route_ctx.route.as_ref(), activity_ctx);

            if let Some(distance_limit) = tour_distance_limit {
                let curr_dis = route_ctx.state.get_route_state(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
                let total_distance = curr_dis + change_distance;
                if distance_limit < total_distance {
                    return stop(self.distance_code);
                }
            }

            if let Some(duration_limit) = tour_duration_limit {
                let curr_dur = route_ctx.state.get_route_state(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);
                let total_duration = curr_dur + change_duration;
                if duration_limit < total_duration {
                    return stop(self.duration_code);
                }
            }
        }

        None
    }
}

impl TravelHardActivityConstraint {
    fn calculate_travel(&self, route: &Route, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        let prev = activity_ctx.prev;
        let tar = activity_ctx.target;
        let next = activity_ctx.next;

        let prev_dep = prev.schedule.departure;

        let (prev_to_tar_dis, prev_to_tar_dur) = self.calculate_leg_travel_info(route, prev, tar, prev_dep);
        if next.is_none() {
            return (prev_to_tar_dis, prev_to_tar_dur);
        }

        let next = next.unwrap();
        let tar_dep = prev_dep + prev_to_tar_dur;

        let (prev_to_next_dis, prev_to_next_dur) = self.calculate_leg_travel_info(route, prev, next, prev_dep);
        let (tar_to_next_dis, tar_to_next_dur) = self.calculate_leg_travel_info(route, tar, next, tar_dep);

        (prev_to_tar_dis + tar_to_next_dis - prev_to_next_dis, prev_to_tar_dur + tar_to_next_dur - prev_to_next_dur)
    }

    fn calculate_leg_travel_info(
        &self,
        route: &Route,
        first: &Activity,
        second: &Activity,
        departure: Timestamp,
    ) -> (Distance, Duration) {
        let first_to_second_dis = self.transport.distance(
            route,
            first.place.location,
            second.place.location,
            TravelTime::Departure(departure),
        );
        let first_to_second_dur = self.transport.duration(
            route,
            first.place.location,
            second.place.location,
            TravelTime::Departure(departure),
        );

        let second_arr = departure + first_to_second_dur;
        let second_wait = (second.place.time.start - second_arr).max(0.);
        let second_dep = second_arr + second_wait + second.place.duration;

        (first_to_second_dis, second_dep - departure)
    }
}
