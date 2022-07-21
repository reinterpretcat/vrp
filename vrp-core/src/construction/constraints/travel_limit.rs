use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::solution::*;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// No travel limits for any actor.
#[derive(Default)]
pub struct NoTravelLimits {}

impl TravelLimits for NoTravelLimits {
    fn tour_distance(&self, _: &Route) -> Option<Distance> {
        None
    }

    fn tour_duration(&self, _: &Route) -> Option<Duration> {
        None
    }

    fn trip_distance(&self, _: &Route, _: usize) -> Option<Distance> {
        None
    }

    fn trip_duration(&self, _: &Route, _: usize) -> Option<Duration> {
        None
    }
}

/// A simple travel limits implementation.
pub struct SimpleTravelLimits {
    distance: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
    duration: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
}

impl SimpleTravelLimits {
    /// Creates a new instance of `SimpleTravelLimits`.
    pub fn new(
        distance: Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>,
        duration: Arc<dyn Fn(&Actor) -> Option<Duration> + Send + Sync>,
    ) -> Self {
        Self { distance, duration }
    }
}

impl TravelLimits for SimpleTravelLimits {
    fn tour_distance(&self, route: &Route) -> Option<Distance> {
        self.distance.deref()(route.actor.as_ref())
    }

    fn tour_duration(&self, route: &Route) -> Option<Duration> {
        self.duration.deref()(route.actor.as_ref())
    }

    fn trip_distance(&self, _: &Route, _: usize) -> Option<Distance> {
        None
    }

    fn trip_duration(&self, _: &Route, _: usize) -> Option<Duration> {
        None
    }
}

/// A module which controls travel limits.
pub struct TravelLimitModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for TravelLimitModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

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
    pub fn new(transport: Arc<dyn TransportCost + Send + Sync>, distance_code: i32, duration_code: i32) -> Self {
        Self {
            state_keys: Vec::default(),
            constraints: vec![ConstraintVariant::HardActivity(Arc::new(TravelHardActivityConstraint {
                distance_code,
                duration_code,
                transport,
            }))],
        }
    }
}

/// A hard activity constraint which allows to limit actor's traveling distance and time.
struct TravelHardActivityConstraint {
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
        let tour_distance_limit = self.transport.limits().tour_distance(route_ctx.route.as_ref());
        let tour_duration_limit = self.transport.limits().tour_duration(route_ctx.route.as_ref());

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
