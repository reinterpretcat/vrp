//! An experimental feature which provides way to insert recharge stations in the tour to recharge
//! (refuel) vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/recharge_test.rs"]
mod recharge_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Route;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::once;
use std::sync::Arc;

/// Specifies dependencies needed to use recharge feature.
pub trait RechargeAspects: Clone + Send + Sync {
    /// Checks whether the job is a recharge job and it can be assigned to the given route.
    fn belongs_to_route(&self, route: &Route, job: &Job) -> bool;

    /// Checks whether the single job is recharge job.
    fn is_recharge_single(&self, single: &Single) -> bool;

    /// Gets recharge keys.
    fn get_state_keys(&self) -> &RechargeKeys;

    /// Gets recharge distance limit function.
    fn get_distance_limit_fn(&self) -> RechargeDistanceLimitFn;

    /// Returns violation code.
    fn get_violation_code(&self) -> ViolationCode;
}

/// Specifies a distance limit function for recharge. It should return a fixed value for the same
/// actor all the time.
pub type RechargeDistanceLimitFn = Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>;
type RechargeSingleFn = Arc<dyn Fn(&Single) -> bool + Send + Sync>;

/// Keeps track of state keys used by the recharge feature.
#[derive(Clone, Debug)]
pub struct RechargeKeys {
    /// A distance limit key.
    pub distance: StateKey,
    /// A recharge station interval key.
    pub intervals: StateKey,
}

impl RechargeKeys {
    fn iter(&self) -> impl Iterator<Item = StateKey> {
        once(self.distance).chain(once(self.intervals))
    }
}

/// Creates a feature to insert charge stations along the route.
pub fn create_recharge_feature<T: RechargeAspects + 'static>(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    aspects: T,
) -> Result<Feature, GenericError> {
    let recharge_keys = aspects.get_state_keys();
    let distance_key = recharge_keys.distance;
    let intervals_key = recharge_keys.intervals;
    let code = aspects.get_violation_code();

    let distance_limit_fn = aspects.get_distance_limit_fn();
    let recharge_single_fn: RechargeSingleFn = {
        let aspects = aspects.clone();
        Arc::new(move |single| aspects.is_recharge_single(single))
    };

    create_multi_trip_feature(
        name,
        recharge_keys.iter().collect(),
        code,
        MarkerInsertionPolicy::Any,
        Arc::new(RechargeableMultiTrip {
            route_intervals: RouteIntervals::Multiple {
                is_marker_single_fn: recharge_single_fn.clone(),
                is_new_interval_needed_fn: Arc::new({
                    let distance_limit_fn = distance_limit_fn.clone();
                    move |route_ctx| {
                        route_ctx
                            .route()
                            .tour
                            .end_idx()
                            .map(|end_idx| {
                                let current: Distance = route_ctx
                                    .state()
                                    .get_activity_state(distance_key, end_idx)
                                    .copied()
                                    .unwrap_or_default();

                                (distance_limit_fn)(route_ctx.route().actor.as_ref())
                                    .map_or(false, |threshold| current > threshold)
                            })
                            .unwrap_or(false)
                    }
                }),
                is_obsolete_interval_fn: Arc::new({
                    let distance_limit_fn = distance_limit_fn.clone();
                    let transport = transport.clone();
                    let get_counter = move |route_ctx: &RouteContext, activity_idx: usize| {
                        route_ctx
                            .state()
                            .get_activity_state(distance_key, activity_idx)
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
                        let end_idx = get_end_idx(route_ctx, right.end);

                        let new_distance = get_counter(route_ctx, left.end) + get_counter(route_ctx, end_idx)
                            - get_counter(route_ctx, right.start + 1)
                            + get_distance(route_ctx.route(), left.end, right.start + 1);

                        (distance_limit_fn)(route_ctx.route().actor.as_ref())
                            .map_or(false, |threshold| compare_floats(new_distance, threshold) != Ordering::Greater)
                    }
                }),
                is_assignable_fn: Arc::new(move |route, job| aspects.belongs_to_route(route, job)),
                intervals_key,
            },
            transport,
            code,
            distance_key,
            distance_limit_fn,
            recharge_single_fn,
        }),
    )
}

struct RechargeableMultiTrip {
    route_intervals: RouteIntervals,
    transport: Arc<dyn TransportCost + Send + Sync>,
    code: ViolationCode,
    distance_key: StateKey,
    distance_limit_fn: RechargeDistanceLimitFn,
    recharge_single_fn: RechargeSingleFn,
}

impl MultiTrip for RechargeableMultiTrip {
    fn get_route_intervals(&self) -> &RouteIntervals {
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
        let mut distance_counters = vec![Distance::default(); route_ctx.route().tour.total()];

        marker_intervals.into_iter().for_each(|(start_idx, end_idx)| {
            let route = route_ctx.route();

            let end_idx = if end_idx != last_idx { end_idx + 1 } else { end_idx };

            let _ = route
                .tour
                .activities_slice(start_idx, end_idx)
                .windows(2)
                .enumerate()
                .filter_map(|(leg_idx, leg)| match leg {
                    [prev, next] => Some((start_idx + leg_idx, prev, next)),
                    _ => None,
                })
                .fold(Distance::default(), |acc, (activity_idx, prev, next)| {
                    let distance = self.transport.distance(
                        route,
                        prev.place.location,
                        next.place.location,
                        TravelTime::Departure(prev.schedule.departure),
                    );
                    let counter = acc + distance;
                    let next_idx = activity_idx + 1;

                    distance_counters[next_idx] = counter;

                    counter
                });
        });

        route_ctx.state_mut().put_activity_states(self.distance_key, distance_counters);
    }

    fn try_recover(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], _: &[Job]) -> bool {
        let routes = &mut solution_ctx.routes;

        let jobs: HashSet<_> = if route_indices.is_empty() {
            solution_ctx
                .ignored
                .iter()
                .filter(|job| job.as_single().map_or(false, |single| (self.recharge_single_fn)(single)))
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
            solution_ctx.required.extend(jobs);

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

        let interval_distance = self
            .route_intervals
            .resolve_marker_intervals(route_ctx)
            .find(|(_, end_idx)| activity_ctx.index <= *end_idx)
            .map(|(_, end_idx)| get_end_idx(route_ctx, end_idx))
            .map(|end_idx| self.get_distance(route_ctx, end_idx))
            .expect("invalid markers state");

        let is_new_recharge =
            activity_ctx.target.job.as_ref().map_or(false, |single| (self.recharge_single_fn)(single));

        let is_violation = if is_new_recharge {
            let ((prev_to_tar_distance, tar_to_next_distance), _) =
                calculate_travel(route_ctx, activity_ctx, self.transport.as_ref());

            // S ----- A ---- [X] ------ B ----- F

            let current_distance = self.get_distance(route_ctx, activity_ctx.index);
            // check S->X
            let is_begin_violates = (current_distance + prev_to_tar_distance) > threshold;
            // check X->F
            let is_end_violates = if activity_ctx.next.is_some() {
                let next_distance = self.get_distance(route_ctx, activity_ctx.index + 1);
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
    fn get_distance(&self, route_ctx: &RouteContext, activity_idx: usize) -> Distance {
        route_ctx
            .state()
            .get_activity_state::<Distance>(self.distance_key, activity_idx)
            .copied()
            .unwrap_or(Distance::default())
    }
}

fn get_end_idx(route_ctx: &RouteContext, end_idx: usize) -> usize {
    let last_idx = route_ctx.route().tour.total() - 1;
    end_idx + if end_idx == last_idx { 0 } else { 1 }
}
