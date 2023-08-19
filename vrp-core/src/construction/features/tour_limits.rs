//! A features to put some extra limits on tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_limits_test.rs"]
mod tour_limits_test;

use super::*;
use crate::construction::enablers::calculate_travel_delta;
use crate::models::common::{Distance, Duration};
use crate::models::problem::{Actor, TransportCost};
use crate::models::solution::Route;

/// A function which returns activity size limit for given actor.
pub type ActivitySizeResolver = Arc<dyn Fn(&Actor) -> Option<usize> + Sync + Send>;
/// A function to resolve travel limit.
pub type TravelLimitFn<T> = Arc<dyn Fn(&Actor) -> Option<T> + Send + Sync>;

/// Creates a limit for activity amount in a tour.
/// This is a hard constraint.
pub fn create_activity_limit_feature(
    name: &str,
    code: ViolationCode,
    limit_func: ActivitySizeResolver,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(ActivityLimitConstraint { code, limit_fn: limit_func })
        .build()
}

/// Creates a travel limits such as distance and/or duration.
/// This is a hard constraint.
pub fn create_travel_limit_feature(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(TravelLimitConstraint {
            distance_code,
            duration_code,
            transport,
            tour_distance_limit_fn,
            tour_duration_limit_fn: tour_duration_limit_fn.clone(),
        })
        .with_state(TravelLimitState { tour_duration_limit_fn, state_keys: vec![] })
        .build()
}

struct ActivityLimitConstraint {
    code: ViolationCode,
    limit_fn: ActivitySizeResolver,
}

impl FeatureConstraint for ActivityLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                (self.limit_fn)(route_ctx.route().actor.as_ref()).and_then(|limit| {
                    let tour_activities = route_ctx.route().tour.job_activity_count();

                    let job_activities = match job {
                        Job::Single(_) => 1,
                        Job::Multi(multi) => multi.jobs.len(),
                    };

                    if tour_activities + job_activities > limit {
                        ConstraintViolation::fail(self.code)
                    } else {
                        ConstraintViolation::success()
                    }
                })
            }
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
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
}

impl TravelLimitConstraint {
    fn calculate_travel(&self, route: &Route, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        calculate_travel_delta(route, activity_ctx, self.transport.as_ref())
    }
}

impl FeatureConstraint for TravelLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx } => {
                let tour_distance_limit = (self.tour_distance_limit_fn)(route_ctx.route().actor.as_ref());
                let tour_duration_limit = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref());

                if tour_distance_limit.is_some() || tour_duration_limit.is_some() {
                    let (change_distance, change_duration) = self.calculate_travel(route_ctx.route(), activity_ctx);

                    if let Some(distance_limit) = tour_distance_limit {
                        let curr_dis = route_ctx.state().get_route_state(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
                        let total_distance = curr_dis + change_distance;
                        if distance_limit < total_distance {
                            return ConstraintViolation::skip(self.distance_code);
                        }
                    }

                    if let Some(duration_limit) = tour_duration_limit {
                        let curr_dur = route_ctx.state().get_route_state(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);
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
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    state_keys: Vec<StateKey>,
}

impl FeatureState for TravelLimitState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(limit_duration) = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref()) {
            route_ctx.state_mut().put_route_state(LIMIT_DURATION_KEY, limit_duration);
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}
