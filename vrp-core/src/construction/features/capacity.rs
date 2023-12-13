//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Activity;
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
        MarkerInsertionPolicy::Last,
        Arc::new(CapacitatedMultiTrip::<T> { route_intervals, code, phantom: Default::default() }),
    )
}

/// Creates capacity feature as a hard constraint.
pub fn create_capacity_limit_feature<T: LoadOps>(name: &str, code: ViolationCode) -> Result<Feature, GenericError> {
    // TODO theoretically, the code can be easily refactored to get opt-out from no-op multi-trip runtime overhead here
    create_multi_trip_feature(
        name,
        code,
        &[CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY, MAX_LOAD_KEY],
        MarkerInsertionPolicy::Last,
        Arc::new(CapacitatedMultiTrip::<T> {
            route_intervals: Arc::new(NoRouteIntervals::default()),
            code,
            phantom: Default::default(),
        }),
    )
    .map(|feature| Feature {
        // NOTE: opt-out from objective
        objective: None,
        ..feature
    })
}

impl<T: LoadOps> FeatureConstraint for CapacitatedMultiTrip<T> {
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
}

struct CapacitatedMultiTrip<T: LoadOps> {
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    code: ViolationCode,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> MultiTrip for CapacitatedMultiTrip<T> {
    fn get_route_intervals(&self) -> &(dyn RouteIntervals) {
        self.route_intervals.as_ref()
    }

    fn get_constraint(&self) -> &(dyn FeatureConstraint) {
        self
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        let marker_intervals = self
            .get_route_intervals()
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
                let (current, _) = route.tour.activities_slice(start_idx, end_idx).iter().enumerate().fold(
                    (start_delivery, T::default()),
                    |(current, max), (idx, activity)| {
                        let activity_idx = start_idx + idx;
                        let change = get_demand(activity).map(|demand| demand.change()).unwrap_or_else(T::default);

                        let current = current + change;
                        let max = max.max_load(current);

                        state.put_activity_state(CURRENT_CAPACITY_KEY, activity_idx, current);
                        state.put_activity_state(MAX_PAST_CAPACITY_KEY, activity_idx, max);

                        (current, max)
                    },
                );

                let current_max = (start_idx..=end_idx).rev().fold(current, |max, activity_idx| {
                    let max = max.max_load(*state.get_activity_state(CURRENT_CAPACITY_KEY, activity_idx).unwrap());
                    state.put_activity_state(MAX_FUTURE_CAPACITY_KEY, activity_idx, max);
                    max
                });

                (current - end_pickup, current_max.max_load(max))
            });

        if let Some(capacity) = route_ctx.route().actor.clone().vehicle.dimens.get_capacity() {
            route_ctx.state_mut().put_route_state(MAX_LOAD_KEY, max_load.ratio(capacity));
        }
    }

    fn try_recover(&self, _: &mut SolutionContext, _: &[usize], _: &[Job]) -> bool {
        // TODO try to recover if multi-trip is used
        false
    }
}

impl<T: LoadOps> CapacitatedMultiTrip<T> {
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
                activity_ctx.index,
                route_ctx.route().actor.vehicle.dimens.get_capacity(),
                demand,
                !self.has_markers(route_ctx),
            )
        };

        violation.map(|stopped| ConstraintViolation { code: self.code, stopped })
    }

    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.route_intervals.get_marker_intervals(route_ctx).map_or(false, |intervals| intervals.len() > 1)
    }

    fn can_handle_demand_on_intervals(
        &self,
        route_ctx: &RouteContext,
        demand: Option<&Demand<T>>,
        insert_idx: Option<usize>,
    ) -> bool {
        let has_demand_violation = |activity_idx: usize| {
            has_demand_violation(
                route_ctx.state(),
                activity_idx,
                route_ctx.route().actor.vehicle.dimens.get_capacity(),
                demand,
                true,
            )
        };

        let has_demand_violation_on_borders = |start_idx: usize, end_idx: usize| {
            has_demand_violation(start_idx).is_none() || has_demand_violation(end_idx).is_none()
        };

        self.route_intervals
            .get_marker_intervals(route_ctx)
            .map(|intervals| {
                if let Some(insert_idx) = insert_idx {
                    intervals
                        .iter()
                        .filter(|(_, end_idx)| insert_idx <= *end_idx)
                        .all(|(start_idx, _)| has_demand_violation(insert_idx.max(*start_idx)).is_none())
                } else {
                    intervals.iter().any(|(start_idx, end_idx)| has_demand_violation_on_borders(*start_idx, *end_idx))
                }
            })
            .unwrap_or_else(|| {
                if let Some(insert_idx) = insert_idx {
                    has_demand_violation(insert_idx).is_none()
                } else {
                    has_demand_violation_on_borders(0, route_ctx.route().tour.total().max(1) - 1)
                }
            })
    }
}

fn has_demand_violation<T: LoadOps>(
    state: &RouteState,
    pivot_idx: usize,
    capacity: Option<&T>,
    demand: Option<&Demand<T>>,
    stopped: bool,
) -> Option<bool> {
    let demand = demand?;
    let capacity = if let Some(capacity) = capacity.copied() {
        capacity
    } else {
        return Some(stopped);
    };

    // check how static delivery affect past max load
    if demand.delivery.0.is_not_empty() {
        let past: T = state.get_activity_state(MAX_PAST_CAPACITY_KEY, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(past + demand.delivery.0)) {
            return Some(stopped);
        }
    }

    // check how static pickup affect future max load
    if demand.pickup.0.is_not_empty() {
        let future: T = state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + demand.pickup.0)) {
            return Some(false);
        }
    }

    // check dynamic load change
    let change = demand.change();
    if change.is_not_empty() {
        let future: T = state.get_activity_state(MAX_FUTURE_CAPACITY_KEY, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + change)) {
            return Some(false);
        }

        let current: T = state.get_activity_state(CURRENT_CAPACITY_KEY, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(current + change)) {
            return Some(false);
        }
    }

    None
}

fn get_demand<T: LoadOps>(activity: &Activity) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| job.dimens.get_demand())
}
