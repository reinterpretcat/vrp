//! A features to put some extra limits on tour.

use super::*;
use crate::construction::constraints::{LIMIT_DURATION_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::models::common::{Distance, Duration, Timestamp};
use crate::models::problem::{Actor, TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};
use std::ops::Deref;

/// A function which returns activity size limit for given actor.
pub type ActivitySizeResolver = Arc<dyn Fn(&Actor) -> Option<usize> + Sync + Send>;
/// A function to resolve travel limit.
pub type TravelLimitFn<T> = Arc<dyn Fn(&Actor) -> Option<T> + Send + Sync>;

/// Creates a limit for activity amount in a tour.
/// This is a hard constraint.
pub fn create_activity_limit(code: ViolationCode, limit_func: ActivitySizeResolver) -> Result<Feature, String> {
    FeatureBuilder::default().with_constraint(Arc::new(ActivityLimitConstraint { code, limit_func })).build()
}

/// Creates a travel limits such as distance and/or duration.
/// This is a hard constraint.
pub fn crete_travel_limit(
    transport: Arc<dyn TransportCost + Send + Sync>,
    tour_distance_limit: TravelLimitFn<Distance>,
    tour_duration_limit: TravelLimitFn<Duration>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_constraint(Arc::new(TravelLimitConstraint {
            distance_code,
            duration_code,
            transport,
            tour_distance_limit,
            tour_duration_limit: tour_duration_limit.clone(),
        }))
        .with_state(Arc::new(TravelLimitState { tour_duration_limit, state_keys: vec![] }))
        .build()
}

struct ActivityLimitConstraint {
    code: ViolationCode,
    limit_func: ActivitySizeResolver,
}

impl FeatureConstraint for ActivityLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.limit_func.deref()(route_ctx.route.actor.as_ref())
                .and_then(|limit| {
                    let tour_activities = route_ctx.route.tour.job_activity_count();

                    let job_activities = match job {
                        Job::Single(_) => 1,
                        Job::Multi(multi) => multi.jobs.len(),
                    };

                    if tour_activities + job_activities > limit {
                        ConstraintViolation::fail(self.code)
                    } else {
                        ConstraintViolation::success()
                    }
                }),
            MoveContext::Activity { .. } => ConstraintViolation::success(),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

struct TravelLimitConstraint {
    distance_code: ViolationCode,
    duration_code: ViolationCode,
    transport: Arc<dyn TransportCost + Send + Sync>,
    tour_distance_limit: TravelLimitFn<Distance>,
    tour_duration_limit: TravelLimitFn<Duration>,
}

impl TravelLimitConstraint {
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

impl FeatureConstraint for TravelLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx } => {
                let tour_distance_limit = self.tour_distance_limit.deref()(route_ctx.route.actor.as_ref());
                let tour_duration_limit = self.tour_duration_limit.deref()(route_ctx.route.actor.as_ref());

                if tour_distance_limit.is_some() || tour_duration_limit.is_some() {
                    let (change_distance, change_duration) =
                        self.calculate_travel(route_ctx.route.as_ref(), activity_ctx);

                    if let Some(distance_limit) = tour_distance_limit {
                        let curr_dis = route_ctx.state.get_route_state(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
                        let total_distance = curr_dis + change_distance;
                        if distance_limit < total_distance {
                            return ConstraintViolation::skip(self.distance_code);
                        }
                    }

                    if let Some(duration_limit) = tour_duration_limit {
                        let curr_dur = route_ctx.state.get_route_state(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);
                        let total_duration = curr_dur + change_duration;
                        if duration_limit < total_duration {
                            return ConstraintViolation::skip(self.duration_code);
                        }
                    }
                }

                None
            }
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

struct TravelLimitState {
    tour_duration_limit: TravelLimitFn<Duration>,
    state_keys: Vec<StateKey>,
}

impl FeatureState for TravelLimitState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(limit_duration) = self.tour_duration_limit.deref()(route_ctx.route.actor.as_ref()) {
            route_ctx.state_mut().put_route_state(LIMIT_DURATION_KEY, limit_duration);
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}
