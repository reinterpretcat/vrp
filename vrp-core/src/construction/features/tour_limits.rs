//! A features to put some extra limits on tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_limits_test.rs"]
mod tour_limits_test;

use std::cmp::Ordering;

use super::*;
use crate::construction::enablers::*;
use crate::models::common::{Distance, Duration};
use crate::models::problem::{Actor, TransportCost};

/// A function which returns activity size limit for a given actor.
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
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(TravelLimitConstraint {
            transport: transport.clone(),
            tour_distance_limit_fn,
            tour_duration_limit_fn: tour_duration_limit_fn.clone(),
            distance_code,
            duration_code,
        })
        .with_state(TravelLimitState { tour_duration_limit_fn, transport, activity })
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
    transport: Arc<dyn TransportCost>,
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
}

impl TravelLimitConstraint {
    fn calculate_travel(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        calculate_travel_delta(route_ctx, activity_ctx, self.transport.as_ref())
    }
}

impl FeatureConstraint for TravelLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx, .. } => {
                let tour_distance_limit = (self.tour_distance_limit_fn)(route_ctx.route().actor.as_ref());
                let tour_duration_limit = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref());

                if tour_distance_limit.is_some() || tour_duration_limit.is_some() {
                    let (change_distance, change_duration) = self.calculate_travel(route_ctx, activity_ctx);

                    if let Some(distance_limit) = tour_distance_limit {
                        let curr_dis = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
                        let total_distance = curr_dis + change_distance;
                        if distance_limit < total_distance {
                            return ConstraintViolation::skip(self.distance_code);
                        }
                    }

                    if let Some(duration_limit) = tour_duration_limit {
                        let curr_dur = route_ctx.state().get_total_duration().copied().unwrap_or(0.);
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
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
}

impl FeatureState for TravelLimitState {
    fn notify_failure(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool {
        let has_empty_routes_with_limit = route_indices
            .iter()
            .filter(|&&idx| solution_ctx.routes[idx].state().get_limit_duration().is_some())
            .any(|&idx| solution_ctx.routes[idx].route().tour.job_count() == 0);

        // skip if we already have empty routes with limit to prevent the algorithm to stuck
        if has_empty_routes_with_limit {
            return false;
        }

        // find an available actor with duration limits
        let Some((route, actor, start_place)) = solution_ctx
            .registry
            .next_route()
            .filter(|route_ctx| (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref()).is_some())
            .map(|route_ctx| route_ctx.route())
            .filter_map(|route| route.actor.detail.start.clone().map(|start| (route, route.actor.clone(), start)))
            .next()
        else {
            return false;
        };

        // find departure time for a job that could potentially be served
        // NOTE: assume that jobs are reshuffled time to time to avoid bias.
        let Some(new_departure_time) = jobs
            .iter()
            .flat_map(|job| job.places())
            .filter_map(|place| {
                place.location.map(|location| {
                    place
                        .times
                        .iter()
                        // consider only jobs with time windows
                        .filter_map(|time_span| time_span.as_time_window())
                        .map(move |tw| (tw, location))
                })
            })
            .flatten()
            // filter out jobs which cannot be assigned due to actor's shift time constraint (naive)
            .filter(|(tw, _)| actor.detail.time.contains(tw.start) || actor.detail.time.contains(tw.end))
            .filter_map(|(job_tw, job_loc)| {
                let duration = self.transport.duration_approx(&actor.vehicle.profile, start_place.location, job_loc);

                // consider multiple possible departure times
                [
                    job_tw.end - duration,                                      // latest possible
                    job_tw.start - duration,                                    // earliest possible
                    job_tw.start - duration + (job_tw.end - job_tw.start) / 2., // middle
                ]
                .into_iter()
                // do not depart outside allowed time
                .filter(|&departure_time| {
                    let start_latest = start_place.time.latest.unwrap_or(f64::MAX);
                    let end_latest = actor.detail.end.as_ref().and_then(|place| place.time.latest).unwrap_or(f64::MAX);

                    start_latest.total_cmp(&departure_time) != Ordering::Less
                        && end_latest.total_cmp(&departure_time) != Ordering::Less
                })
                .find(|&departure_time| {
                    // check job can be served with this departure
                    let earliest_departure = start_place.time.earliest.unwrap_or(0.0).max(departure_time);
                    let travel_info = TravelTime::Departure(departure_time);
                    let travel_duration = self.transport.duration(route, start_place.location, job_loc, travel_info);

                    earliest_departure + travel_duration <= job_tw.end
                })
            })
            .next()
        else {
            return false;
        };

        // get route, reschedule it and add to the solution
        let Some(mut route_ctx) = solution_ctx.registry.get_route(&actor) else {
            return false;
        };
        update_route_departure(&mut route_ctx, self.activity.as_ref(), self.transport.as_ref(), new_departure_time);
        solution_ctx.routes.push(route_ctx);

        true
    }

    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(limit_duration) = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref()) {
            route_ctx.state_mut().set_limit_duration(limit_duration);
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}
}
