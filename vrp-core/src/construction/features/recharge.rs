//! An experimental feature which provides a way to insert recharge stations in the tour to recharge
//! (refuel) vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/recharge_test.rs"]
mod recharge_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Route;
use std::collections::HashSet;
use std::sync::Arc;

/// Provides a way to build the recharge/refuel feature.
#[allow(clippy::type_complexity)]
pub struct RechargeFeatureBuilder {
    name: String,
    violation_code: Option<ViolationCode>,
    transport: Option<Arc<dyn TransportCost>>,
    belongs_to_route_fn: Option<Arc<dyn Fn(&Route, &Job) -> bool + Send + Sync>>,
    is_recharge_single_fn: Option<RechargeSingleFn>,
    distance_limit_fn: Option<RechargeDistanceLimitFn>,
}

impl RechargeFeatureBuilder {
    /// Creates a new instance of `RechargeFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            violation_code: None,
            is_recharge_single_fn: None,
            belongs_to_route_fn: None,
            distance_limit_fn: None,
            transport: None,
        }
    }

    /// Sets constraint violation code which is used to report back the reason of job's unassignment.
    pub fn set_violation_code(mut self, violation_code: ViolationCode) -> Self {
        self.violation_code = Some(violation_code);
        self
    }

    /// Sets transport costs to estimate distance.
    pub fn set_transport(mut self, transport: Arc<dyn TransportCost>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Sets a function which specifies whether a given single job can be considered as a recharge job.
    pub fn set_is_recharge_single<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> bool + Send + Sync + 'static,
    {
        self.is_recharge_single_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which specifies whether a given route can serve a given job. This function
    /// should return false, if the job is not recharge.
    pub fn set_belongs_to_route<F>(mut self, func: F) -> Self
    where
        F: Fn(&Route, &Job) -> bool + Send + Sync + 'static,
    {
        self.belongs_to_route_fn = Some(Arc::new(func));
        self
    }

    /// Specifies a distance limit function for recharge. It should return a fixed value for the same
    /// actor all the time.
    pub fn set_distance_limit<F>(mut self, func: F) -> Self
    where
        F: Fn(&Actor) -> Option<Distance> + Send + Sync + 'static,
    {
        self.distance_limit_fn = Some(Arc::new(func));
        self
    }

    /// Builds the recharge feature if all dependencies are set.
    pub fn build(&mut self) -> GenericResult<Feature> {
        let is_marker_single_fn =
            self.is_recharge_single_fn.take().ok_or_else(|| GenericError::from("is_reload_single must be set"))?;
        let is_assignable_fn =
            self.belongs_to_route_fn.take().ok_or_else(|| GenericError::from("belongs_to_route must be set"))?;

        let transport = self.transport.take().ok_or_else(|| GenericError::from("transport must be set"))?;
        let distance_limit_fn =
            self.distance_limit_fn.take().ok_or_else(|| GenericError::from("distance_limit must be set"))?;

        let code = self.violation_code.unwrap_or_default();

        create_multi_trip_feature(
            self.name.as_str(),
            code,
            MarkerInsertionPolicy::Any,
            Arc::new(RechargeableMultiTrip {
                route_intervals: RouteIntervals::Multiple {
                    is_marker_single_fn: is_marker_single_fn.clone(),
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
                                        .get_recharge_distance_at(end_idx)
                                        .copied()
                                        .unwrap_or_default();

                                    (distance_limit_fn)(route_ctx.route().actor.as_ref())
                                        .is_some_and(|threshold| current > threshold)
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
                                .get_recharge_distance_at(activity_idx)
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
                                .is_some_and(|threshold| new_distance <= threshold)
                        }
                    }),
                    is_assignable_fn,
                    intervals_state: Arc::new(RechargeIntervalsState),
                },
                transport,
                code,
                distance_limit_fn,
                recharge_single_fn: is_marker_single_fn.clone(),
            }),
        )
    }
}

type RechargeDistanceLimitFn = Arc<dyn Fn(&Actor) -> Option<Distance> + Send + Sync>;
type RechargeSingleFn = Arc<dyn Fn(&Single) -> bool + Send + Sync>;

custom_route_intervals_state!(RechargeIntervals);
custom_activity_state!(RechargeDistance typeof Distance);

struct RechargeableMultiTrip {
    route_intervals: RouteIntervals,
    transport: Arc<dyn TransportCost>,
    code: ViolationCode,
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

        route_ctx.state_mut().set_recharge_distance_states(distance_counters);
    }

    fn try_recover(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], _: &[Job]) -> bool {
        let routes = &mut solution_ctx.routes;

        let jobs: HashSet<_> = if route_indices.is_empty() {
            solution_ctx
                .ignored
                .iter()
                .filter(|job| job.as_single().is_some_and(|single| (self.recharge_single_fn)(single)))
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
            MoveContext::Activity { route_ctx, activity_ctx, .. } => self.evaluate_activity(route_ctx, activity_ctx),
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

        let is_new_recharge = activity_ctx.target.job.as_ref().is_some_and(|single| (self.recharge_single_fn)(single));

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

        if is_violation { ConstraintViolation::skip(self.code) } else { None }
    }
}

impl RechargeableMultiTrip {
    fn get_distance(&self, route_ctx: &RouteContext, activity_idx: usize) -> Distance {
        route_ctx.state().get_recharge_distance_at(activity_idx).copied().unwrap_or(Distance::default())
    }
}

fn get_end_idx(route_ctx: &RouteContext, end_idx: usize) -> usize {
    let last_idx = route_ctx.route().tour.total() - 1;
    end_idx + if end_idx == last_idx { 0 } else { 1 }
}
