//! Provides way to insert recharge stations in the tour to recharge (refuel) vehicle.

use super::*;
use crate::construction::enablers::{get_shift_index, get_vehicle_id_from_job, is_correct_vehicle, JobTie};
use std::marker::PhantomData;
use std::sync::Arc;
use vrp_core::construction::enablers::{calculate_travel, FixedReloadIntervals, RouteIntervals};
use vrp_core::construction::features::{create_multi_trip_feature, MultiTrip, RECHARGE_INTERVALS_KEY};
use vrp_core::models::solution::Route;

/// Creates a feature to insert charge stations along the route.
pub fn create_recharge_feature<T: LoadOps>(
    name: &str,
    code: ViolationCode,
    distance_limit: (StateKey, Distance),
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<Feature, GenericError> {
    const DISTANCE_THRESHOLD_RATIO: f64 = 0.8;
    let (distance_state_key, distance_limit) = distance_limit;

    create_multi_trip_feature(
        name,
        code,
        &[distance_state_key, RECHARGE_INTERVALS_KEY],
        Arc::new(RechargeableMultiTrip::<T> {
            route_intervals: Some(Arc::new(FixedReloadIntervals {
                is_marker_single_fn: Box::new(is_recharge_single),
                is_new_interval_needed_fn: Box::new(move |route_ctx| {
                    route_ctx
                        .route()
                        .tour
                        .end()
                        .map(|end| {
                            let current: Distance = route_ctx
                                .state()
                                .get_activity_state(distance_state_key, end)
                                .copied()
                                .unwrap_or_default();

                            let threshold = distance_limit * DISTANCE_THRESHOLD_RATIO;

                            current > threshold
                        })
                        .unwrap_or(false)
                }),
                is_obsolete_interval_fn: Box::new(|_route_ctx, _left, _right| {
                    // TODO
                    todo!()
                }),
                is_assignable_fn: Box::new(|route, job| {
                    job.as_single().map_or(false, |job| {
                        is_correct_vehicle(route, get_vehicle_id_from_job(job), get_shift_index(&job.dimens))
                    })
                }),
                intervals_key: RECHARGE_INTERVALS_KEY,
            })),
            transport,
            code,
            distance_state_key,
            distance_limit,
            phantom: Default::default(),
        }),
    )
}

impl<T: LoadOps> FeatureConstraint for RechargeableMultiTrip<T> {
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

struct RechargeableMultiTrip<T: LoadOps> {
    route_intervals: Option<Arc<dyn RouteIntervals + Send + Sync>>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    code: ViolationCode,
    distance_state_key: StateKey,
    distance_limit: Distance,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> MultiTrip for RechargeableMultiTrip<T> {}

impl<T: LoadOps> RouteIntervals for RechargeableMultiTrip<T> {
    fn is_marker_job(&self, job: &Job) -> bool {
        self.route_intervals.as_ref().map_or(false, |inner| inner.is_marker_job(job))
    }

    fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool {
        self.route_intervals.as_ref().map_or(false, |inner| inner.is_marker_assignable(route, job))
    }

    fn is_new_interval_needed(&self, route_ctx: &RouteContext) -> bool {
        self.route_intervals.as_ref().map_or(false, |inner| inner.is_new_interval_needed(route_ctx))
    }

    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        self.route_intervals.as_ref().and_then(|inner| inner.get_marker_intervals(route_ctx))
    }

    fn get_interval_key(&self) -> Option<StateKey> {
        self.route_intervals.as_ref().and_then(|inner| inner.get_interval_key())
    }

    fn update_route_intervals(&self, route_ctx: &mut RouteContext) {
        if let Some(route_intervals) = &self.route_intervals {
            route_intervals.update_route_intervals(route_ctx);
        }
        self.recalculate_states(route_ctx);
    }

    fn update_solution_intervals(&self, solution_ctx: &mut SolutionContext) {
        if let Some(route_intervals) = &self.route_intervals {
            route_intervals.update_solution_intervals(solution_ctx);
        }
    }
}

impl<T: LoadOps> RechargeableMultiTrip<T> {
    fn evaluate_job(&self, _: &RouteContext, _: &Job) -> Option<ConstraintViolation> {
        ConstraintViolation::success()
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let is_prev_recharge = activity_ctx.prev.job.as_ref().map_or(false, |job| is_recharge_single(job));
        let current_distance = if is_prev_recharge {
            // NOTE ignore current_distance for prev if prev is marker job as we store
            //      accumulated distance here to simplify obsolete intervals calculations
            Distance::default()
        } else {
            route_ctx
                .state()
                .get_activity_state::<Distance>(self.distance_state_key, activity_ctx.prev)
                .copied()
                .unwrap_or(Distance::default())
        };

        let (prev_to_next_distance, _) = calculate_travel(route_ctx, activity_ctx, self.transport.as_ref());

        if current_distance + prev_to_next_distance > self.distance_limit {
            ConstraintViolation::skip(self.code)
        } else {
            None
        }
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        let marker_intervals = self
            .get_marker_intervals(route_ctx)
            .cloned()
            .unwrap_or_else(|| vec![(0, route_ctx.route().tour.total() - 1)]);

        marker_intervals.into_iter().for_each(|(start_idx, end_idx)| {
            let (route, state) = route_ctx.as_mut();

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
}

fn is_recharge_single(single: &Single) -> bool {
    single.dimens.get_job_type().map_or(false, |t| t == "recharge")
}
