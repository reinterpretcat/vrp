//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Activity;
use std::marker::PhantomData;
use std::sync::Arc;

struct CapacityRouteStateKey<T: LoadOps>(PhantomData<T>);

#[derive(Clone, Copy, Default)]
struct CapacityActivityState<T: LoadOps> {
    current: T,
    max_future: T,
    max_past: T,
}

struct CapacityRouteState<T: LoadOps> {
    activities: Vec<CapacityActivityState<T>>,
    capacity: Option<T>,
}

#[cfg(test)]
pub(crate) trait CurrentCapacityActivityState {
    fn get_current_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T>;
}

pub(crate) trait MaxFutureCapacityActivityState {
    fn get_max_future_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T>;
}

pub(crate) trait MaxPastCapacityActivityState {
    fn get_max_past_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T>;
}

trait CapacityStateAccess {
    fn get_capacity_states<T: LoadOps>(&self) -> Option<&CapacityRouteState<T>>;

    fn prepare_capacity_states<T: LoadOps>(
        &mut self,
        activity_count: usize,
        capacity: Option<T>,
    ) -> &mut [CapacityActivityState<T>];
}

impl CapacityStateAccess for RouteState {
    fn get_capacity_states<T: LoadOps>(&self) -> Option<&CapacityRouteState<T>> {
        self.get_tour_state::<CapacityRouteStateKey<T>, _>()
    }

    fn prepare_capacity_states<T: LoadOps>(
        &mut self,
        activity_count: usize,
        capacity: Option<T>,
    ) -> &mut [CapacityActivityState<T>] {
        let state = self.get_or_init_exclusive_tour_state::<CapacityRouteStateKey<T>, CapacityRouteState<T>>(|| {
            CapacityRouteState { activities: Vec::with_capacity(activity_count), capacity }
        });

        state.activities.clear();
        state.activities.resize(activity_count, CapacityActivityState::default());
        state.capacity = capacity;

        state.activities.as_mut_slice()
    }
}

#[cfg(test)]
impl CurrentCapacityActivityState for RouteState {
    fn get_current_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T> {
        self.get_capacity_states::<T>()?.activities.get(activity_idx).map(|state| &state.current)
    }
}

impl MaxFutureCapacityActivityState for RouteState {
    fn get_max_future_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T> {
        self.get_capacity_states::<T>()?.activities.get(activity_idx).map(|state| &state.max_future)
    }
}

impl MaxPastCapacityActivityState for RouteState {
    fn get_max_past_capacity_at<T: LoadOps>(&self, activity_idx: usize) -> Option<&T> {
        self.get_capacity_states::<T>()?.activities.get(activity_idx).map(|state| &state.max_past)
    }
}

custom_tour_state!(pub(crate) MaxVehicleLoad typeof Float, setter(cfg(test)));

custom_dimension!(pub VehicleCapacity typeof T: LoadOps);

/// A trait to get or set job demand.
pub trait JobDemandDimension {
    /// Sets job demand.
    fn set_job_demand<T: LoadOps>(&mut self, demand: Demand<T>) -> &mut Self;

    /// Gets job demand.
    fn get_job_demand<T: LoadOps>(&self) -> Option<&Demand<T>>;
}

/// Provides a way to build capacity limit feature.
pub struct CapacityFeatureBuilder<T: LoadOps> {
    name: String,
    route_intervals: Option<RouteIntervals>,
    violation_code: Option<ViolationCode>,
    phantom_data: PhantomData<T>,
}

impl<T: LoadOps> CapacityFeatureBuilder<T> {
    /// Creates a new instance of `CapacityFeatureBuilder`
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), route_intervals: None, violation_code: None, phantom_data: Default::default() }
    }

    /// Sets constraint violation code which is used to report back the reason of job's unassignment.
    pub fn set_violation_code(mut self, violation_code: ViolationCode) -> Self {
        self.violation_code = Some(violation_code);
        self
    }

    /// Sets route intervals to trigger multi trip behavior (used with reload flavors).
    pub fn set_route_intervals(mut self, route_intervals: RouteIntervals) -> Self {
        self.route_intervals = Some(route_intervals);
        self
    }

    /// Builds a feature.
    pub fn build(self) -> GenericResult<Feature> {
        let name = self.name.as_str();
        let violation_code = self.violation_code.unwrap_or_default();

        match self.route_intervals {
            Some(route_intervals) => create_multi_trip_feature(
                name,
                violation_code,
                MarkerInsertionPolicy::Last,
                Arc::new(CapacitatedMultiTrip::<T> { route_intervals, violation_code, phantom: Default::default() }),
            ),
            _ => create_multi_trip_feature(
                name,
                violation_code,
                MarkerInsertionPolicy::Last,
                Arc::new(CapacitatedMultiTrip::<T> {
                    route_intervals: RouteIntervals::Single,
                    violation_code,
                    phantom: Default::default(),
                }),
            ),
        }
    }
}

impl<T> FeatureConstraint for CapacitatedMultiTrip<T>
where
    T: LoadOps,
{
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_job(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx, .. } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (&source, &candidate) {
            (Job::Single(s_source), Job::Single(s_candidate)) => {
                let source_demand: Option<&Demand<T>> = s_source.dimens.get_job_demand();
                let candidate_demand: Option<&Demand<T>> = s_candidate.dimens.get_job_demand();

                match (source_demand, candidate_demand) {
                    (None, None) | (Some(_), None) => Ok(source),
                    _ => {
                        let source_demand = source_demand.cloned().unwrap_or_default();
                        let candidate_demand = candidate_demand.cloned().unwrap_or_default();
                        let new_demand = source_demand + candidate_demand;

                        let mut single = Single { places: s_source.places.clone(), dimens: s_source.dimens.clone() };
                        single.dimens.set_job_demand(new_demand);

                        Ok(Job::Single(Arc::new(single)))
                    }
                }
            }
            _ => Err(self.violation_code),
        }
    }
}

struct CapacitatedMultiTrip<T>
where
    T: LoadOps,
{
    route_intervals: RouteIntervals,
    violation_code: ViolationCode,
    phantom: PhantomData<T>,
}

impl<T> MultiTrip for CapacitatedMultiTrip<T>
where
    T: LoadOps,
{
    fn get_route_intervals(&self) -> &RouteIntervals {
        &self.route_intervals
    }

    fn get_constraint(&self) -> &dyn FeatureConstraint {
        self
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        let tour_len = route_ctx.route().tour.total();
        let marker_intervals = self
            .get_route_intervals()
            .get_marker_intervals(route_ctx)
            .cloned()
            .unwrap_or_else(|| vec![(0, tour_len - 1)]);
        let capacity = route_ctx.route().actor.vehicle.dimens.get_vehicle_capacity::<T>().copied();
        let (route, state) = route_ctx.as_mut();
        let capacity_states = state.prepare_capacity_states::<T>(tour_len, capacity);

        let (_, max_load) =
            marker_intervals.into_iter().fold((T::default(), T::default()), |(acc, max), (start_idx, end_idx)| {
                // determine static deliveries loaded at the begin and static pickups brought to the end
                let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                    (acc, T::default()),
                    |acc, activity| {
                        self.get_demand(activity)
                            .map(|demand| (acc.0 + demand.delivery.0, acc.1 + demand.pickup.0))
                            .unwrap_or_else(|| acc)
                    },
                );

                // determine actual load at each activity and max discovered in the past
                let (current, _) = route.tour.activities_slice(start_idx, end_idx).iter().enumerate().fold(
                    (start_delivery, T::default()),
                    |(current, max), (idx, activity)| {
                        let activity_idx = start_idx + idx;
                        let change = self.get_demand(activity).map(|demand| demand.change()).unwrap_or_default();

                        let current = current + change;
                        let max = max.max_load(current);

                        capacity_states[activity_idx].current = current;
                        capacity_states[activity_idx].max_past = max;

                        (current, max)
                    },
                );

                let current_max = (start_idx..=end_idx).rev().fold(current, |max, activity_idx| {
                    let max = max.max_load(capacity_states[activity_idx].current);
                    capacity_states[activity_idx].max_future = max;

                    max
                });

                (current - end_pickup, current_max.max_load(max))
            });

        if let Some(capacity) = capacity {
            state.update_tour_state::<MaxVehicleLoadTourStateKey, _>(max_load.ratio(&capacity));
        }
    }

    fn try_recover(&self, _: &mut SolutionContext, _: &[usize], _: &[Job]) -> bool {
        // TODO try to recover if multi-trip is used
        false
    }
}

impl<T> CapacitatedMultiTrip<T>
where
    T: LoadOps,
{
    fn evaluate_job(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let can_handle = match job {
            Job::Single(job) => self.can_handle_demand_on_intervals(route_ctx, job.dimens.get_job_demand(), None),
            Job::Multi(job) => job
                .jobs
                .iter()
                .any(|job| self.can_handle_demand_on_intervals(route_ctx, job.dimens.get_job_demand(), None)),
        };

        if can_handle { ConstraintViolation::success() } else { ConstraintViolation::fail(self.violation_code) }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let demand = self.get_demand(activity_ctx.target);

        let violation = if activity_ctx.target.has_parent_job() {
            // NOTE multi job has dynamic demand which can go in another interval
            if self.can_handle_demand_on_intervals(route_ctx, demand, Some(activity_ctx.index)) {
                None
            } else {
                Some(false)
            }
        } else {
            has_demand_violation(route_ctx, activity_ctx.index, demand, !self.has_markers(route_ctx))
        };

        violation.map(|stopped| ConstraintViolation { code: self.violation_code, stopped })
    }

    fn has_markers(&self, route_ctx: &RouteContext) -> bool {
        self.route_intervals.get_marker_intervals(route_ctx).is_some_and(|intervals| intervals.len() > 1)
    }

    fn can_handle_demand_on_intervals(
        &self,
        route_ctx: &RouteContext,
        demand: Option<&Demand<T>>,
        insert_idx: Option<usize>,
    ) -> bool {
        let has_demand_violation = |activity_idx: usize| has_demand_violation(route_ctx, activity_idx, demand, true);

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
                    let last_idx = route_ctx.route().tour.end_idx().unwrap_or_default();
                    has_demand_violation_on_borders(0, last_idx)
                }
            })
    }

    fn get_demand<'a>(&self, activity: &'a Activity) -> Option<&'a Demand<T>> {
        activity.job.as_ref().and_then(|single| single.dimens.get_job_demand())
    }
}

fn has_demand_violation<T: LoadOps>(
    route_ctx: &RouteContext,
    pivot_idx: usize,
    demand: Option<&Demand<T>>,
    stopped: bool,
) -> Option<bool> {
    let demand = demand?;
    let capacity_state = route_ctx.state().get_capacity_states::<T>();
    let capacity = match capacity_state {
        Some(state) => state.capacity.as_ref(),
        None => route_ctx.route().actor.vehicle.dimens.get_vehicle_capacity(),
    };

    let capacity = if let Some(capacity) = capacity {
        capacity
    } else {
        return Some(stopped);
    };

    let state = capacity_state.and_then(|states| states.activities.get(pivot_idx));

    // check how static delivery affects a past max load
    if demand.delivery.0.is_not_empty() {
        let past = state.map(|state| state.max_past).unwrap_or_default();
        if !capacity.can_fit(&(past + demand.delivery.0)) {
            return Some(stopped);
        }
    }

    // check how static pickup affect future max load
    if demand.pickup.0.is_not_empty() {
        let future = state.map(|state| state.max_future).unwrap_or_default();
        if !capacity.can_fit(&(future + demand.pickup.0)) {
            return Some(false);
        }
    }

    // Static demand is covered by the past and future load checks above. A dynamic activity can
    // combine both demand types, so keep using the complete change in that case.
    if demand.has_dynamic() {
        let change = demand.change();
        let future = state.map(|state| state.max_future).unwrap_or_default();
        if !capacity.can_fit(&(future + change)) {
            return Some(false);
        }

        let current = state.map(|state| state.current).unwrap_or_default();
        if !capacity.can_fit(&(current + change)) {
            return Some(false);
        }
    }

    None
}

// TODO extend macro to support this.
struct JobDemandDimenKey;
impl JobDemandDimension for Dimensions {
    fn set_job_demand<T: LoadOps>(&mut self, demand: Demand<T>) -> &mut Self {
        self.set_value::<JobDemandDimenKey, _>(demand);
        self
    }

    fn get_job_demand<T: LoadOps>(&self) -> Option<&Demand<T>> {
        self.get_value::<JobDemandDimenKey, _>()
    }
}
