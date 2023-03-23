//! Provides feature to add capacity limitation on a vehicle.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/capacity_test.rs"]
mod capacity_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::problem::Single;
use crate::models::solution::Activity;
use rosomaxa::prelude::Objective;
use std::iter::once;
use std::slice::Iter;
use std::sync::Arc;

/// Creates capacity feature as a hard constraint with multi trip functionality as a soft constraint.
pub fn create_capacity_limit_with_multi_trip_feature<T: LoadOps>(
    name: &str,
    code: ViolationCode,
    multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(CapacityConstraint::new(code, multi_trip.clone()))
        .with_objective(CapacityObjective::new(multi_trip.clone()))
        .with_state(CapacityState::new(code, multi_trip))
        .build()
}

/// Creates capacity feature as a hard constraint.
pub fn create_capacity_limit_feature<T: LoadOps>(name: &str, code: ViolationCode) -> Result<Feature, String> {
    let multi_trip = Arc::new(NoMultiTrip::<T>::default());
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(CapacityConstraint::new(code, multi_trip.clone()))
        .with_state(CapacityState::new(code, multi_trip))
        .build()
}

struct CapacityConstraint<T: LoadOps> {
    code: ViolationCode,
    multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>,
}

impl<T: LoadOps> FeatureConstraint for CapacityConstraint<T> {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        if once(&source).chain(once(&candidate)).any(|job| self.multi_trip.is_marker_job(job)) {
            return Err(self.code);
        }

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

impl<T: LoadOps> CapacityConstraint<T> {
    fn new(code: ViolationCode, multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>) -> Self {
        Self { code, multi_trip }
    }

    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        if self.multi_trip.is_marker_job(job) {
            return if self.multi_trip.is_assignable(&route_ctx.route, job) {
                None
            } else {
                Some(ConstraintViolation { code: self.code, stopped: true })
            };
        };

        let can_handle = match job {
            Job::Single(job) => {
                can_handle_demand_on_intervals(route_ctx, self.multi_trip.as_ref(), job.dimens.get_demand(), None)
            }
            Job::Multi(job) => job.jobs.iter().any(|job| {
                can_handle_demand_on_intervals(route_ctx, self.multi_trip.as_ref(), job.dimens.get_demand(), None)
            }),
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
        if activity_ctx
            .target
            .job
            .as_ref()
            .map_or(false, |job| self.multi_trip.is_marker_job(&Job::Single(job.clone())))
        {
            // NOTE insert reload job in route only as last
            let is_first = activity_ctx.prev.job.is_none();
            let is_not_last = activity_ctx.next.as_ref().and_then(|next| next.job.as_ref()).is_some();

            return if is_first || is_not_last {
                ConstraintViolation::skip(self.code)
            } else {
                ConstraintViolation::success()
            };
        };

        let demand = get_demand(activity_ctx.target);

        let violation = if activity_ctx.target.retrieve_job().map_or(false, |job| job.as_multi().is_some()) {
            // NOTE multi job has dynamic demand which can go in another interval
            if can_handle_demand_on_intervals(route_ctx, self.multi_trip.as_ref(), demand, Some(activity_ctx.index)) {
                None
            } else {
                Some(false)
            }
        } else {
            has_demand_violation(
                &route_ctx.state,
                activity_ctx.prev,
                route_ctx.route.actor.vehicle.dimens.get_capacity(),
                demand,
                !self.multi_trip.has_markers(route_ctx),
            )
        };

        violation.map(|stopped| ConstraintViolation { code: self.code, stopped })
    }
}

struct CapacityObjective<T: LoadOps> {
    multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>,
}

impl<T: LoadOps> CapacityObjective<T> {
    pub fn new(multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>) -> Self {
        Self { multi_trip }
    }

    fn estimate_job(&self, job: &Job) -> Cost {
        if self.multi_trip.is_marker_job(job) {
            -1.
        } else {
            0.
        }
    }
}

impl<T: LoadOps> Objective for CapacityObjective<T> {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route.tour.jobs())
            .map(|job| self.estimate_job(&job))
            .sum()
    }
}

impl<T: LoadOps> FeatureObjective for CapacityObjective<T> {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { job, .. } => self.estimate_job(job),
            MoveContext::Activity { .. } => 0.,
        }
    }
}

struct CapacityState<T: LoadOps> {
    multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>,
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    state_keys: Vec<StateKey>,
    code: ViolationCode,
}

impl<T: LoadOps> CapacityState<T> {
    pub fn new(code: ViolationCode, multi_trip: Arc<dyn MultiTrip<Constraint = T> + Send + Sync>) -> Self {
        let context_transition = Box::new(ConcreteJobContextTransition {
            remove_required: {
                let multi_trip = multi_trip.clone();
                move |_, _, job| multi_trip.is_marker_job(job)
            },
            promote_required: |_, _, _| false,
            remove_locked: |_, _, _| false,
            promote_locked: {
                let multi_trip = multi_trip.clone();
                move |_, _, job| multi_trip.is_marker_job(job)
            },
        });

        Self {
            multi_trip,
            context_transition,
            state_keys: vec![CURRENT_CAPACITY_KEY, MAX_FUTURE_CAPACITY_KEY, MAX_PAST_CAPACITY_KEY, MAX_LOAD_KEY],
            code,
        }
    }

    fn recalculate_states(&self, route_ctx: &mut RouteContext) {
        self.multi_trip.accept_route_state(route_ctx);
        let reload_intervals = self
            .multi_trip
            .get_marker_intervals(route_ctx)
            .cloned()
            .unwrap_or_else(|| vec![(0, route_ctx.route.tour.total() - 1)]);

        let (_, max_load) =
            reload_intervals.into_iter().fold((T::default(), T::default()), |(acc, max), (start_idx, end_idx)| {
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

        if let Some(capacity) = route_ctx.route.actor.clone().vehicle.dimens.get_capacity() {
            route_ctx.state_mut().put_route_state(MAX_LOAD_KEY, max_load.ratio(capacity));
        }
    }
}

impl<T: LoadOps> FeatureState for CapacityState<T> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.accept_route_state(solution_ctx.routes.get_mut(route_index).unwrap());
        self.multi_trip.accept_insertion(solution_ctx, route_index, job, self.code);
    }

    fn accept_route_state(&self, solution_ctx: &mut RouteContext) {
        self.recalculate_states(solution_ctx);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, self.context_transition.as_ref());

        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            self.recalculate_states(route_ctx);
        });

        self.multi_trip.accept_solution_state(solution_ctx);
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
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

fn can_handle_demand_on_intervals<T: LoadOps>(
    ctx: &RouteContext,
    multi_trip: &(dyn MultiTrip<Constraint = T> + Send + Sync),
    demand: Option<&Demand<T>>,
    insert_idx: Option<usize>,
) -> bool {
    let has_demand_violation = |activity: &Activity| {
        has_demand_violation(&ctx.state, activity, ctx.route.actor.vehicle.dimens.get_capacity(), demand, true)
    };

    let has_demand_violation_on_borders = |start_idx: usize, end_idx: usize| {
        has_demand_violation(ctx.route.tour.get(start_idx).unwrap()).is_none()
            || has_demand_violation(ctx.route.tour.get(end_idx).unwrap()).is_none()
    };

    multi_trip
        .get_marker_intervals(ctx)
        .map(|intervals| {
            if let Some(insert_idx) = insert_idx {
                intervals.iter().filter(|(_, end_idx)| insert_idx <= *end_idx).all(|interval| {
                    has_demand_violation(ctx.route.tour.get(insert_idx.max(interval.0)).unwrap()).is_none()
                })
            } else {
                intervals.iter().any(|(start_idx, end_idx)| has_demand_violation_on_borders(*start_idx, *end_idx))
            }
        })
        .unwrap_or_else(|| {
            if let Some(insert_idx) = insert_idx {
                has_demand_violation(ctx.route.tour.get(insert_idx).unwrap()).is_none()
            } else {
                has_demand_violation_on_borders(0, ctx.route.tour.total().max(1) - 1)
            }
        })
}

fn get_demand<T: LoadOps>(activity: &Activity) -> Option<&Demand<T>> {
    activity.job.as_ref().and_then(|job| job.dimens.get_demand())
}
