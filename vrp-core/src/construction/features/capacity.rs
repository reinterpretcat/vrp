//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Activity;
use std::marker::PhantomData;
use std::sync::Arc;

custom_activity_state!(pub(crate) CurrentCapacity typeof T: LoadOps);

custom_activity_state!(pub(crate) MaxFutureCapacity typeof T: LoadOps);

custom_activity_state!(pub(crate) MaxPastCapacity typeof T: LoadOps);

custom_tour_state!(pub(crate) MaxVehicleLoad typeof Float);

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

        if let Some(route_intervals) = self.route_intervals {
            create_multi_trip_feature(
                name,
                violation_code,
                MarkerInsertionPolicy::Last,
                Arc::new(CapacitatedMultiTrip::<T> { route_intervals, violation_code, phantom: Default::default() }),
            )
        } else {
            create_multi_trip_feature(
                name,
                violation_code,
                MarkerInsertionPolicy::Last,
                Arc::new(CapacitatedMultiTrip::<T> {
                    route_intervals: RouteIntervals::Single,
                    violation_code,
                    phantom: Default::default(),
                }),
            )
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

        let (_, max_load) =
            marker_intervals.into_iter().fold((T::default(), T::default()), |(acc, max), (start_idx, end_idx)| {
                let route = route_ctx.route();

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

        route_ctx.state_mut().set_current_capacity_states(current_capacities);
        route_ctx.state_mut().set_max_past_capacity_states(max_past_capacities);
        route_ctx.state_mut().set_max_future_capacity_states(max_future_capacities);

        if let Some(capacity) = route_ctx.route().actor.clone().vehicle.dimens.get_vehicle_capacity::<T>() {
            route_ctx.state_mut().set_max_vehicle_load(max_load.ratio(capacity));
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

        if can_handle {
            ConstraintViolation::success()
        } else {
            ConstraintViolation::fail(self.violation_code)
        }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let demand = self.get_demand(activity_ctx.target);

        let violation = if activity_ctx.target.retrieve_job().is_some_and(|job| job.as_multi().is_some()) {
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
    let capacity: Option<&T> = route_ctx.route().actor.vehicle.dimens.get_vehicle_capacity();
    let demand = demand?;

    let capacity = if let Some(capacity) = capacity {
        capacity
    } else {
        return Some(stopped);
    };

    let state = route_ctx.state();

    // check how static delivery affects a past max load
    if demand.delivery.0.is_not_empty() {
        let past: T = state.get_max_past_capacity_at(pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(past + demand.delivery.0)) {
            return Some(stopped);
        }
    }

    // check how static pickup affect future max load
    if demand.pickup.0.is_not_empty() {
        let future: T = state.get_max_future_capacity_at(pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + demand.pickup.0)) {
            return Some(false);
        }
    }

    // check dynamic load change
    let change = demand.change();
    if change.is_not_empty() {
        let future: T = state.get_max_future_capacity_at(pivot_idx).copied().unwrap_or_default();
        if !capacity.can_fit(&(future + change)) {
            return Some(false);
        }

        let current: T = state.get_current_capacity_at(pivot_idx).copied().unwrap_or_default();
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
