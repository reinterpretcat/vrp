//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::{Activity, Route};
use std::marker::PhantomData;
use std::sync::Arc;

/// Creates capacity feature as a hard constraint with multi trip functionality as a soft constraint.
pub fn create_capacity_limit_with_multi_trip_feature<T: LoadOps>(
    name: &str,
    code: ViolationCode,
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
) -> Result<Feature, GenericError> {
    create_multi_trip_feature(
        name,
        code,
        &[CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY, MAX_LOAD_KEY],
        Arc::new(CapacitatedRouteIntervals::<T> { inner: Some(route_intervals), code, phantom: Default::default() }),
    )
}

/// Creates capacity feature as a hard constraint.
pub fn create_capacity_limit_feature<T: LoadOps>(name: &str, code: ViolationCode) -> Result<Feature, GenericError> {
    create_multi_trip_feature(
        name,
        code,
        &[CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY, MAX_LOAD_KEY],
        Arc::new(CapacitatedRouteIntervals::<T> { inner: None, code, phantom: Default::default() }),
    )
    .map(|feature| Feature {
        // NOTE: opt-out from objective
        objective: None,
        ..feature
    })
}

struct CapacitatedRouteIntervals<T: LoadOps> {
    inner: Option<Arc<dyn RouteIntervals + Send + Sync>>,
    code: ViolationCode,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> RouteIntervals for CapacitatedRouteIntervals<T> {
    fn is_marker_job(&self, job: &Job) -> bool {
        self.inner.as_ref().map_or(false, |inner| inner.is_marker_job(job))
    }

    fn is_marker_assignable(&self, route: &Route, job: &Job) -> bool {
        self.inner.as_ref().map_or(false, |inner| inner.is_marker_assignable(route, job))
    }

    fn is_new_interval_needed(&self, route_ctx: &RouteContext) -> bool {
        self.inner.as_ref().map_or(false, |inner| inner.is_new_interval_needed(route_ctx))
    }

    fn get_marker_intervals<'a>(&self, route_ctx: &'a RouteContext) -> Option<&'a Vec<(usize, usize)>> {
        self.inner.as_ref().and_then(|inner| inner.get_marker_intervals(route_ctx))
    }

    fn get_interval_key(&self) -> Option<StateKey> {
        self.inner.as_ref().and_then(|inner| inner.get_interval_key())
    }

    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_job(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (&source, &candidate) {
            (Job::Single(s_source), Job::Single(s_candidate)) => {
                let source_demand: Option<&Demand<T>> = s_source.dimens.get_demand();
                let candidate_demand: Option<&Demand<T>> = s_candidate.dimens.get_demand();

                match (source_demand, candidate_demand) {
                    (None, None) | (Some(_), None) => Ok(source),
                    _ => {
                        let source_demand = source_demand.cloned().unwrap_or_default();
                        let candidate_demand = candidate_demand.cloned().unwrap_or_default();
                        let new_demand = source_demand + candidate_demand;

                        let mut dimens = s_source.dimens.clone();
                        dimens.set_demand(new_demand);

                        Ok(Job::Single(Arc::new(Single { places: s_source.places.clone(), dimens })))
                    }
                }
            }
            _ => Err(self.code),
        }
    }

    fn update_route_intervals(&self, route_ctx: &mut RouteContext) {
        if let Some(inner) = &self.inner {
            inner.update_route_intervals(route_ctx);
        }
        self.recalculate_states(route_ctx);
    }

    fn update_solution_intervals(&self, solution_ctx: &mut SolutionContext) {
        if let Some(inner) = &self.inner {
            inner.update_solution_intervals(solution_ctx);
        }
    }
}

impl<T: LoadOps> CapacitatedRouteIntervals<T> {
    fn evaluate_job(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let can_handle = match job {
            Job::Single(job) => self.can_handle_demand_on_intervals(route_ctx, job.dimens.get_demand(), None),
            Job::Multi(job) => {
                job.jobs.iter().any(|job| self.can_handle_demand_on_intervals(route_ctx, job.dimens.get_demand(), None))
            }
        };

        if can_handle {
            ConstraintViolation::success()
        } else {
            ConstraintViolation::fail(self.code)
        }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let demand = get_demand(activity_ctx.target);

        let violation = if activity_ctx.target.retrieve_job().map_or(false, |job| job.as_multi().is_some()) {
            // NOTE multi job has dynamic demand which can go in another interval
            if self.can_handle_demand_on_intervals(route_ctx, demand, Some(activity_ctx.index)) {
                None
            } else {
                Some(false)
            }
        } else {
            has_demand_violation(
                route_ctx.state(),
                activity_ctx.prev,
                route_ctx.route().actor.vehicle.dimens.get_capacity(),
                demand,
                !self.has_markers(route_ctx),
            )
        };

        violation.map(|stopped| ConstraintViolation { code: self.code, stopped })
    }

    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.get_marker_intervals(route_ctx).map_or(false, |intervals| intervals.len() > 1)
    }

    fn can_handle_demand_on_intervals(
        &self,
        route_ctx: &RouteContext,
        demand: Option<&Demand<T>>,
        insert_idx: Option<usize>,
    ) -> bool {
        let has_demand_violation = |activity: &Activity| {
            has_demand_violation(
                route_ctx.state(),
                activity,
                route_ctx.route().actor.vehicle.dimens.get_capacity(),
                demand,
                true,
            )
        };

        let has_demand_violation_on_borders = |start_idx: usize, end_idx: usize| {
            has_demand_violation(route_ctx.route().tour.get(start_idx).unwrap()).is_none()
                || has_demand_violation(route_ctx.route().tour.get(end_idx).unwrap()).is_none()
        };

        self.get_marker_intervals(route_ctx)
            .map(|intervals| {
                if let Some(insert_idx) = insert_idx {
                    intervals.iter().filter(|(_, end_idx)| insert_idx <= *end_idx).all(|interval| {
                        has_demand_violation(route_ctx.route().tour.get(insert_idx.max(interval.0)).unwrap()).is_none()
                    })
                } else {
                    intervals.iter().any(|(start_idx, end_idx)| has_demand_violation_on_borders(*start_idx, *end_idx))
                }
            })
            .unwrap_or_else(|| {
                if let Some(insert_idx) = insert_idx {
                    has_demand_violation(route_ctx.route().tour.get(insert_idx).unwrap()).is_none()
                } else {
                    has_demand_violation_on_borders(0, route_ctx.route().tour.total().max(1) - 1)
                }
            })
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        let marker_intervals = self
            .get_marker_intervals(route_ctx)
            .cloned()
            .unwrap_or_else(|| vec![(0, route_ctx.route().tour.total() - 1)]);

        let (_, max_load) =
            marker_intervals.into_iter().fold((T::default(), T::default()), |(acc, max), (start_idx, end_idx)| {
                let (route, state) = route_ctx.as_mut();

                // determine static deliveries loaded at the begin and static pickups brought to the end
                let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                    (acc, T::default()),
                    |acc, activity| {
                        get_demand(activity)
                            .map(|demand| (acc.0 + demand.delivery.0, acc.1 + demand.pickup.0))
                            .unwrap_or_else(|| acc)
                    },
                );

                // determine actual load at each activity and max discovered in the past
                let (current, _) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                    (start_delivery, T::default()),
                    |(current, max), activity| {
                        let change = get_demand(activity).map(|demand| demand.change()).unwrap_or_else(T::default);

                        let current = current + change;
                        let max = max.max_load(current);

                        state.put_activity_state(CURRENT_CAPACITY_KEY, activity, current);
                        state.put_activity_state(MAX_PAST_CAPACITY_KEY, activity, max);

                        (current, max)
                    },
                );

                let current_max =
                    route.tour.activities_slice(start_idx, end_idx).iter().rev().fold(current, |max, activity| {
                        let max = max.max_load(*state.get_activity_state(CURRENT_CAPACITY_KEY, activity).unwrap());
                        state.put_activity_state(MAX_FUTURE_CAPACITY_KEY, activity, max);
                        max
                    });

                (current - end_pickup, current_max.max_load(max))
            });

        if let Some(capacity) = route_ctx.route().actor.clone().vehicle.dimens.get_capacity() {
            route_ctx.state_mut().put_route_state(MAX_LOAD_KEY, max_load.ratio(capacity));
        }
    }
}

fn has_demand_violation<T: LoadOps>(
    state: &RouteState,
    pivot: &Activity,
    capacity: Option<&T>,
    demand: Option<&Demand<T>>,
    stopped: bool,
) -> Option<bool> {
    if let Some(demand) = demand {
        if let Some(&capacity) = capacity {
            let default = T::default();

            // check how static delivery affect past max load
            if demand.delivery.0.is_not_empty() {
                let past = *state.get_activity_state(MAX_PAST_CAPACITY_KEY, pivot).unwrap_or(&default);
                if !capacity.can_fit(&(past + demand.delivery.0)) {
                    return Some(stopped);
                }
            }

            // check how static pickup affect future max load
            if demand.pickup.0.is_not_empty() {
                let future = *state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot).unwrap_or(&default);
                if !capacity.can_fit(&(future + demand.pickup.0)) {
                    return Some(false);
                }
            }

            // check dynamic load change
            let change = demand.change();
            if change.is_not_empty() {
                let future = *state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot).unwrap_or(&default);
                if !capacity.can_fit(&(future + change)) {
                    return Some(false);
                }

                let current = *state.get_activity_state(CURRENT_CAPACITY_KEY, pivot).unwrap_or(&default);
                if !capacity.can_fit(&(current + change)) {
                    return Some(false);
                }
            }

            None
        } else {
            Some(stopped)
        }
    } else {
        None
    }
}

fn get_demand<T: LoadOps>(activity: &Activity) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| job.dimens.get_demand())
}
