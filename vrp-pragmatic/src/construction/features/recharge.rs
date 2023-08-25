//! Provides way to insert recharge stations in the tour to recharge (refuel) vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/recharge_test.rs"]
mod recharge_test;

use super::*;
use crate::construction::enablers::*;
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::sync::Arc;
use vrp_core::construction::enablers::*;
use vrp_core::construction::features::*;
use vrp_core::models::solution::{Activity, Route};

/// Specifies a distance limit function for recharge. It should return a fixed value for the same
/// actor all the time.
pub type RechargeDistanceLimitFn = Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>;

/// Creates a feature to insert charge stations along the route.
pub fn create_recharge_feature(
    name: &str,
    code: ViolationCode,
    distance_limit_fn: RechargeDistanceLimitFn,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<Feature, GenericError> {
    create_multi_trip_feature(
        name,
        code,
        &[RECHARGE_DISTANCE_KEY, RECHARGE_INTERVALS_KEY],
        MarkerInsertionPolicy::Any,
        Arc::new(RechargeableMultiTrip {
            route_intervals: FixedRouteIntervals {
                is_marker_single_fn: Box::new(is_recharge_single),
                is_new_interval_needed_fn: Box::new({
                    let distance_limit_fn = distance_limit_fn.clone();
                    move |route_ctx| {
                        route_ctx
                            .route()
                            .tour
                            .end()
                            .map(|end| {
                                let current: Distance = route_ctx
                                    .state()
                                    .get_activity_state(RECHARGE_DISTANCE_KEY, end)
                                    .copied()
                                    .unwrap_or_default();

                                (distance_limit_fn)(route_ctx.route().actor.as_ref())
                                    .map_or(false, |threshold| current > threshold)
                            })
                            .unwrap_or(false)
                    }
                }),
                is_obsolete_interval_fn: Box::new({
                    let distance_limit_fn = distance_limit_fn.clone();
                    let transport = transport.clone();
                    let get_counter = move |route_ctx: &RouteContext, activity_idx: usize| {
                        route_ctx
                            .route()
                            .tour
                            .get(activity_idx)
                            .and_then(|activity| route_ctx.state().get_activity_state(RECHARGE_DISTANCE_KEY, activity))
                            .copied()
                            .unwrap_or(Distance::default())
                    };
                    let get_distance = move |route: &Route, from_idx: usize, to_idx: usize| {
                        route.tour.get(from_idx).zip(route.tour.get(to_idx)).map_or(
                            Distance::default(),
                            |(from, to)| {
                                transport.distance(
                                    route,
                                    from.place.location,
                                    to.place.location,
                                    TravelTime::Departure(from.schedule.departure),
                                )
                            },
                        )
                    };
                    move |route_ctx, left, right| {
                        let new_distance = get_counter(route_ctx, left.end) + get_counter(route_ctx, right.end)
                            - get_counter(route_ctx, right.start + 1)
                            + get_distance(route_ctx.route(), left.end, right.start + 1);

                        (distance_limit_fn)(route_ctx.route().actor.as_ref())
                            .map_or(false, |threshold| compare_floats(new_distance, threshold) != Ordering::Greater)
                    }
                }),
                is_assignable_fn: Box::new(|route, job| {
                    job.as_single().map_or(false, |job| {
                        is_correct_vehicle(route, get_vehicle_id_from_job(job), get_shift_index(&job.dimens))
                    })
                }),
                intervals_key: RECHARGE_INTERVALS_KEY,
            },
            transport,
            code,
            distance_state_key: RECHARGE_DISTANCE_KEY,
            distance_limit_fn,
        }),
    )
}

struct RechargeableMultiTrip {
    route_intervals: FixedRouteIntervals,
    transport: Arc<dyn TransportCost + Send + Sync>,
    code: ViolationCode,
    distance_state_key: StateKey,
    distance_limit_fn: RechargeDistanceLimitFn,
}

impl MultiTrip for RechargeableMultiTrip {
    fn get_route_intervals(&self) -> &(dyn RouteIntervals) {
        &self.route_intervals
    }

    fn get_constraint(&self) -> &(dyn FeatureConstraint) {
        self
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        if (self.distance_limit_fn)(route_ctx.route().actor.as_ref()).is_none() {
            return;
        }

        let last_idx = route_ctx.route().tour.total() - 1;
        let marker_intervals = self.route_intervals.resolve_marker_intervals(route_ctx).collect::<Vec<_>>();

        marker_intervals.into_iter().for_each(|(start_idx, end_idx)| {
            let (route, state) = route_ctx.as_mut();

            let end_idx = if end_idx != last_idx { end_idx + 1 } else { end_idx };

            let _ = route
                .tour
                .activities_slice(start_idx, end_idx)
                .windows(2)
                .filter_map(|leg| match leg {
                    [prev, next] => Some((prev, next)),
                    _ => None,
                })
                .fold(Distance::default(), |acc, (prev, next)| {
                    let distance = self.transport.distance(
                        route,
                        prev.place.location,
                        next.place.location,
                        TravelTime::Departure(prev.schedule.departure),
                    );
                    let counter = acc + distance;

                    state.put_activity_state(self.distance_state_key, next, counter);

                    counter
                });
        });
    }

    fn try_recover(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], _: &[Job]) -> bool {
        let routes = &mut solution_ctx.routes;

        let jobs: HashSet<_> = if route_indices.is_empty() {
            solution_ctx
                .ignored
                .iter()
                .filter(|job| job.as_single().map_or(false, |single| is_recharge_single(single.as_ref())))
                .cloned()
                .collect()
        } else {
            routes
                .iter()
                .enumerate()
                .filter(|(idx, _)| route_indices.contains(idx))
                .flat_map(|(_, route_ctx)| {
                    solution_ctx
                        .ignored
                        .iter()
                        .filter(|job| self.route_intervals.is_marker_assignable(route_ctx.route(), job))
                })
                .cloned()
                .collect()
        };

        if jobs.is_empty() {
            false
        } else {
            solution_ctx.ignored.retain(|job| !jobs.contains(job));
            solution_ctx.locked.extend(jobs.iter().cloned());
            solution_ctx.required.extend(jobs.into_iter());

            true
        }
    }
}

impl FeatureConstraint for RechargeableMultiTrip {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_job(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

impl RechargeableMultiTrip {
    fn evaluate_job(&self, _: &RouteContext, _: &Job) -> Option<ConstraintViolation> {
        ConstraintViolation::success()
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let threshold = (self.distance_limit_fn)(route_ctx.route().actor.as_ref())?;

        let end = self
            .route_intervals
            .resolve_marker_intervals(route_ctx)
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .and_then(|(_, end_idx)| route_ctx.route().tour.get(end_idx))
            .expect("invalid markers state");
        let interval_distance = self.get_distance(route_ctx, end);

        let is_new_recharge = activity_ctx.target.job.as_ref().map_or(false, |job| is_recharge_single(job));

        let is_violation = if is_new_recharge {
            let ((prev_to_tar_distance, tar_to_next_distance), _) =
                calculate_travel(route_ctx, activity_ctx, self.transport.as_ref());

            // S ----- A ---- [X] ------ B ----- F

            let current_distance = self.get_distance(route_ctx, activity_ctx.prev);
            // check S->X
            let is_begin_violates = (current_distance + prev_to_tar_distance) > threshold;
            // check X->F
            let is_end_violates = if let Some(next) = &activity_ctx.next {
                let next_distance = self.get_distance(route_ctx, next);
                let new_interval_distance = interval_distance - next_distance;

                (new_interval_distance + tar_to_next_distance) > threshold
            } else {
                false
            };

            is_begin_violates || is_end_violates
        } else {
            let (distance_delta, _) = calculate_travel_delta(route_ctx, activity_ctx, self.transport.as_ref());

            (interval_distance + distance_delta) > threshold
        };

        if is_violation {
            ConstraintViolation::skip(self.code)
        } else {
            None
        }
    }
}

impl RechargeableMultiTrip {
    fn get_distance(&self, route_ctx: &RouteContext, activity: &Activity) -> Distance {
        route_ctx
            .state()
            .get_activity_state::<Distance>(self.distance_state_key, activity)
            .copied()
            .unwrap_or(Distance::default())
    }
}

fn is_recharge_single(single: &Single) -> bool {
    single.dimens.get_job_type().map_or(false, |t| t == "recharge")
}
