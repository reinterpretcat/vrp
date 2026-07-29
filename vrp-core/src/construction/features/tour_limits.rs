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

/// Creates a minimum limit for activity amount in a tour.
/// This is a soft constraint (objective) that penalizes solutions where routes have fewer activities than the minimum.
/// Routes with zero activities (empty routes) are allowed.
/// The penalty helps guide the solver toward solutions that meet the minimum, while still allowing
/// exploration of solutions that don't meet the minimum during the search.
pub fn create_min_activity_limit_feature(
    name: &str,
    min_limit_fn: ActivitySizeResolver,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default().with_name(name).with_objective(MinActivityLimitObjective { min_limit_fn }).build()
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

/// Objective that penalizes routes with fewer activities than the minimum limit.
/// This guides the solver toward valid solutions while still allowing exploration.
struct MinActivityLimitObjective {
    min_limit_fn: ActivitySizeResolver,
}

/// Penalty carried by a single route for staying below the minimum activity count.
///
/// Squared rather than linear on purpose. With a linear penalty the objective is a plain sum of
/// deficits, which makes it blind to redistribution: moving a job from one under-sized route to
/// another lowers one deficit by 1 and raises the other by 1, so the sum does not move and the
/// search has no reason to consolidate. Squaring makes the same move an improvement whenever it
/// evens the routes out, which is the behaviour the limit is meant to express.
///
/// Empty routes are not penalized — they carry no tour at all.
fn min_activity_penalty(activity_count: usize, min_limit: usize) -> Cost {
    if activity_count == 0 || activity_count >= min_limit {
        return Cost::default();
    }

    let deficit = (min_limit - activity_count) as Cost;

    deficit * deficit
}

impl FeatureObjective for MinActivityLimitObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.routes.iter().fold(0., |acc, route_ctx| {
            match (self.min_limit_fn)(route_ctx.route().actor.as_ref()) {
                Some(min_limit) => acc + min_activity_penalty(route_ctx.route().tour.job_activity_count(), min_limit),
                None => acc,
            }
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        // Report the marginal penalty change of putting this job into this route, so that a route
        // which is still below the minimum is preferred over one that is not — and over opening yet
        // another route. Returning a constant here (as before) left the layer without any influence
        // on route choice at all.
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => (self.min_limit_fn)(route_ctx.route().actor.as_ref())
                .map(|min_limit| {
                    let current = route_ctx.route().tour.job_activity_count();
                    let added = match job {
                        Job::Single(_) => 1,
                        Job::Multi(multi) => multi.jobs.len(),
                    };

                    min_activity_penalty(current + added, min_limit) - min_activity_penalty(current, min_limit)
                })
                .unwrap_or_default(),
            MoveContext::Activity { .. } => Cost::default(),
        }
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

    /// Returns the idle stretch in front of the first job of an otherwise empty route, which the
    /// duration limit must not be charged for.
    ///
    /// An empty route departs at its shift's `earliest`, and `advance_departure_time` bails out on
    /// a tour without jobs (`departure_time.rs`), so it cannot run during insertion evaluation.
    /// The travel delta, however, already contains the waiting time (`calculate_travel_leg`), so a
    /// full-day shift charges everything between the shift start and the job's time window against
    /// the limit — and a job whose window opens late in the day can then never open a tour, even
    /// though the route it would form is trivially short.
    ///
    /// Reclaiming that stretch is not a relaxation: moving the departure forward by it is always
    /// legal on a virgin route, because there is no other activity whose schedule could break, and
    /// a break (`TimeSpan::Offset`) cannot be present either. Routes that already carry a job are
    /// left alone — there the shift is bounded by the existing activities.
    fn reclaimable_leading_wait(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Duration {
        if route_ctx.route().tour.job_count() != 0 {
            return Duration::default();
        }

        let route = route_ctx.route();
        let prev = activity_ctx.prev;
        let departure = prev.schedule.departure;

        let travel = self.transport.duration(
            route,
            prev.place.location,
            activity_ctx.target.place.location,
            TravelTime::Departure(departure),
        );

        let latest_departure =
            route.actor.detail.start.as_ref().and_then(|start| start.time.latest).unwrap_or(Float::MAX);

        (activity_ctx.target.place.time.start - departure - travel).max(0.).min(latest_departure - departure)
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
                        let total_duration =
                            curr_dur + change_duration - self.reclaimable_leading_wait(route_ctx, activity_ctx);
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
                        // but not max ones
                        .filter(|tw| *tw != TimeWindow::max())
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
