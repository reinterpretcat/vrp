//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Activity;
use std::iter::once;
use std::marker::PhantomData;
use std::sync::Arc;

/// Specifies all needed feature keys.
#[derive(Clone, Debug)]
pub struct CapacityKeys {
    /// State keys.
    pub state_keys: CapacityStateKeys,
    /// Dimension keys
    pub dimen_keys: CapacityDimenKeys,
}

/// Combines all keys needed for capacity feature usage.
#[derive(Clone, Debug)]
pub struct CapacityStateKeys {
    /// A key which tracks current vehicle capacity.
    pub current_capacity: StateKey,
    /// A key which tracks maximum vehicle capacity ahead in route.
    pub max_future_capacity: StateKey,
    /// A key which tracks maximum capacity backward in route.
    pub max_past_capacity: StateKey,
    /// A key which tracks max load in tour.
    pub max_load: StateKey,
}

impl From<&mut StateKeyRegistry> for CapacityStateKeys {
    fn from(state_registry: &mut StateKeyRegistry) -> Self {
        Self {
            current_capacity: state_registry.next_key(),
            max_future_capacity: state_registry.next_key(),
            max_past_capacity: state_registry.next_key(),
            max_load: state_registry.next_key(),
        }
    }
}

impl CapacityStateKeys {
    pub(crate) fn iter(&self) -> impl Iterator<Item = StateKey> {
        once(self.current_capacity)
            .chain(once(self.max_future_capacity))
            .chain(once(self.max_past_capacity))
            .chain(once(self.max_load))
    }
}

/// Dimension keys for capacity feature.
#[derive(Clone, Debug)]
pub struct CapacityDimenKeys {
    /// Vehicle capacity key.
    pub vehicle_capacity: DimenKey,
    /// Activity demand key.
    pub activity_demand: DimenKey,
}

impl From<&mut DimenKeyRegistry> for CapacityDimenKeys {
    fn from(registry: &mut DimenKeyRegistry) -> Self {
        Self {
            vehicle_capacity: registry.next_key(DimenScope::Vehicle),
            activity_demand: registry.next_key(DimenScope::Activity),
        }
    }
}

/// Creates capacity feature as a hard constraint with multi trip functionality as a soft constraint.
pub fn create_capacity_limit_with_multi_trip_feature<T: LoadOps>(
    name: &str,
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    feature_keys: CapacityKeys,
    capacity_code: ViolationCode,
) -> Result<Feature, GenericError> {
    create_multi_trip_feature(
        name,
        feature_keys.state_keys.clone(),
        capacity_code,
        MarkerInsertionPolicy::Last,
        Arc::new(CapacitatedMultiTrip::<T> {
            route_intervals,
            feature_keys,
            capacity_code,
            phantom: Default::default(),
        }),
    )
}

/// Creates capacity feature as a hard constraint.
pub fn create_capacity_limit_feature<T: LoadOps>(
    name: &str,
    feature_keys: CapacityKeys,
    capacity_code: ViolationCode,
) -> Result<Feature, GenericError> {
    // TODO theoretically, the code can be easily refactored to get opt-out from no-op multi-trip runtime overhead here
    create_multi_trip_feature(
        name,
        feature_keys.state_keys.clone(),
        capacity_code,
        MarkerInsertionPolicy::Last,
        Arc::new(CapacitatedMultiTrip::<T> {
            route_intervals: Arc::new(NoRouteIntervals::default()),
            feature_keys,
            capacity_code,
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
                let demand_key = self.feature_keys.dimen_keys.activity_demand;
                let source_demand: Option<&Demand<T>> = get_job_demand(s_source, demand_key);
                let candidate_demand: Option<&Demand<T>> = get_job_demand(s_candidate, demand_key);

                match (source_demand, candidate_demand) {
                    (None, None) | (Some(_), None) => Ok(source),
                    _ => {
                        let source_demand = source_demand.cloned().unwrap_or_default();
                        let candidate_demand = candidate_demand.cloned().unwrap_or_default();
                        let new_demand = source_demand + candidate_demand;

                        let mut dimens = s_source.dimens.clone();
                        dimens.set_value(demand_key, new_demand);

                        Ok(Job::Single(Arc::new(Single { places: s_source.places.clone(), dimens })))
                    }
                }
            }
            _ => Err(self.capacity_code),
        }
    }
}

struct CapacitatedMultiTrip<T: LoadOps> {
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    feature_keys: CapacityKeys,
    capacity_code: ViolationCode,
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

        let tour_len = route_ctx.route().tour.total();

        let mut current_capacities = vec![T::default(); tour_len];
        let mut max_past_capacities = vec![T::default(); tour_len];
        let mut max_future_capacities = vec![T::default(); tour_len];

        let demand_key = self.feature_keys.dimen_keys.activity_demand;
        let capacity_key = self.feature_keys.dimen_keys.vehicle_capacity;

        let (_, max_load) =
            marker_intervals.into_iter().fold((T::default(), T::default()), |(acc, max), (start_idx, end_idx)| {
                let route = route_ctx.route();

                // determine static deliveries loaded at the begin and static pickups brought to the end
                let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                    (acc, T::default()),
                    |acc, activity| {
                        get_activity_demand(activity, demand_key)
                            .map(|demand| (acc.0 + demand.delivery.0, acc.1 + demand.pickup.0))
                            .unwrap_or_else(|| acc)
                    },
                );

                // determine actual load at each activity and max discovered in the past
                let (current, _) = route.tour.activities_slice(start_idx, end_idx).iter().enumerate().fold(
                    (start_delivery, T::default()),
                    |(current, max), (idx, activity)| {
                        let activity_idx = start_idx + idx;
                        let change = get_activity_demand(activity, demand_key)
                            .map(|demand| demand.change())
                            .unwrap_or_else(T::default);

                        let current = current + change;
                        let max = max.max_load(current);

                        current_capacities[activity_idx] = current;
                        max_past_capacities[activity_idx] = max;

                        (current, max)
                    },
                );

                let current_max = (start_idx..=end_idx).rev().fold(current, |max, activity_idx| {
                    let max = max.max_load(current_capacities[activity_idx]);
                    max_future_capacities[activity_idx] = max;

                    max
                });

                (current - end_pickup, current_max.max_load(max))
            });

        let state_keys = &self.feature_keys.state_keys;

        route_ctx.state_mut().put_activity_states(state_keys.current_capacity, current_capacities);
        route_ctx.state_mut().put_activity_states(state_keys.max_past_capacity, max_past_capacities);
        route_ctx.state_mut().put_activity_states(state_keys.max_future_capacity, max_future_capacities);

        if let Some(capacity) = route_ctx.route().actor.vehicle.dimens.get_capacity(capacity_key).copied() {
            route_ctx.state_mut().put_route_state(state_keys.max_load, max_load.ratio(&capacity));
        }
    }

    fn try_recover(&self, _: &mut SolutionContext, _: &[usize], _: &[Job]) -> bool {
        // TODO try to recover if multi-trip is used
        false
    }
}

impl<T: LoadOps> CapacitatedMultiTrip<T> {
    fn evaluate_job(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let demand_key = self.feature_keys.dimen_keys.activity_demand;
        let can_handle = match job {
            Job::Single(job) => self.can_handle_demand_on_intervals(route_ctx, get_job_demand(job, demand_key), None),
            Job::Multi(job) => job
                .jobs
                .iter()
                .any(|job| self.can_handle_demand_on_intervals(route_ctx, get_job_demand(job, demand_key), None)),
        };

        if can_handle {
            ConstraintViolation::success()
        } else {
            ConstraintViolation::fail(self.capacity_code)
        }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let demand = get_activity_demand(activity_ctx.target, self.feature_keys.dimen_keys.activity_demand);

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
                get_capacity(route_ctx, self.feature_keys.dimen_keys.vehicle_capacity),
                demand,
                &self.feature_keys.state_keys,
                !self.has_markers(route_ctx),
            )
        };

        violation.map(|stopped| ConstraintViolation { code: self.capacity_code, stopped })
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
        let capacity = get_capacity(route_ctx, self.feature_keys.dimen_keys.vehicle_capacity);

        let has_demand_violation = |activity_idx: usize| {
            has_demand_violation(route_ctx.state(), activity_idx, capacity, demand, &self.feature_keys.state_keys, true)
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
    state_keys: &CapacityStateKeys,
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
        let past: T = state.get_activity_state(state_keys.max_past_capacity, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(past + demand.delivery.0)) {
            return Some(stopped);
        }
    }

    // check how static pickup affect future max load
    if demand.pickup.0.is_not_empty() {
        let future: T =
            state.get_activity_state(state_keys.max_future_capacity, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + demand.pickup.0)) {
            return Some(false);
        }
    }

    // check dynamic load change
    let change = demand.change();
    if change.is_not_empty() {
        let future: T =
            state.get_activity_state(state_keys.max_future_capacity, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + change)) {
            return Some(false);
        }

        let current: T = state.get_activity_state(state_keys.current_capacity, pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(current + change)) {
            return Some(false);
        }
    }

    None
}

fn get_activity_demand<T: LoadOps>(activity: &Activity, demand_key: DimenKey) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| get_job_demand(job.as_ref(), demand_key))
}

fn get_job_demand<T: LoadOps>(single: &Single, demand_key: DimenKey) -> Option<&Demand<T>> {
    single.dimens.get_demand(demand_key)
}

fn get_capacity<T: LoadOps>(route_ctx: &RouteContext, capacity_key: DimenKey) -> Option<&T> {
    route_ctx.route().actor.vehicle.dimens.get_capacity(capacity_key)
}
