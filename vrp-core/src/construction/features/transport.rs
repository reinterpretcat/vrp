//! Provides the way to deal time/distance cost.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/transport_test.rs"]
mod transport_test;

use super::*;
use crate::construction::enablers::{update_route_schedule, ScheduleStateKeys};
use crate::models::common::Timestamp;
use crate::models::problem::{ActivityCost, Single, TransportCost, TravelTime};
use crate::models::solution::Activity;
use std::ops::Deref;

// TODO
//  remove get_total_cost, get_route_costs, get_max_cost methods from contexts
//  add validation rule which ensures usage of only one of these methods.

/// Creates a travel costs feature which considers distance and duration for minimization.
pub fn create_minimize_transport_costs_feature(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    time_window_code: ViolationCode,
) -> Result<Feature, String> {
    create_feature(
        name,
        transport,
        activity,
        time_window_code,
        Box::new(|insertion_ctx| insertion_ctx.solution.get_total_cost()),
    )
}

/// Creates a travel costs feature which considers duration for minimization as global objective.
/// NOTE: distance costs is still considered on local level.
pub fn create_minimize_duration_feature(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    time_window_code: ViolationCode,
) -> Result<Feature, String> {
    create_feature(
        name,
        transport,
        activity,
        time_window_code,
        Box::new(|insertion_ctx| {
            insertion_ctx.solution.routes.iter().fold(Cost::default(), move |acc, route_ctx| {
                acc + route_ctx.state().get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.)
            })
        }),
    )
}

/// Creates a travel costs feature which considers distance for minimization as global objective.
/// NOTE: duration costs is still considered on local level.
pub fn create_minimize_distance_feature(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    time_window_code: ViolationCode,
) -> Result<Feature, String> {
    create_feature(
        name,
        transport,
        activity,
        time_window_code,
        Box::new(|insertion_ctx| {
            insertion_ctx.solution.routes.iter().fold(Cost::default(), move |acc, route_ctx| {
                acc + route_ctx.state().get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.)
            })
        }),
    )
}

fn create_feature(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    time_window_code: ViolationCode,
    fitness_fn: Box<dyn Fn(&InsertionContext) -> f64 + Send + Sync>,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(TransportConstraint {
            code: time_window_code,
            transport: transport.clone(),
            activity: activity.clone(),
        })
        .with_state(TransportState::new(transport.clone(), activity.clone()))
        .with_objective(TransportObjective { activity, transport, fitness_fn })
        .build()
}

struct TransportConstraint {
    code: ViolationCode,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
}

impl TransportConstraint {
    fn evaluate_job(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let date = route_ctx.route().tour.start().unwrap().schedule.departure;
        let check_single = |single: &Arc<Single>| {
            single
                .places
                .iter()
                .flat_map(|place| place.times.iter())
                .any(|time| time.intersects(date, &route_ctx.route().actor.detail.time))
        };

        let has_time_intersection = match job {
            Job::Single(single) => check_single(single),
            Job::Multi(multi) => multi.jobs.iter().all(check_single),
        };

        if has_time_intersection {
            None
        } else {
            ConstraintViolation::fail(self.code)
        }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let actor = route_ctx.route().actor.as_ref();
        let route = route_ctx.route();

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let departure = prev.schedule.departure;

        if actor.detail.time.end < prev.place.time.start
            || actor.detail.time.end < target.place.time.start
            || next.map_or(false, |next| actor.detail.time.end < next.place.time.start)
        {
            return ConstraintViolation::fail(self.code);
        }

        let (next_act_location, latest_arr_time_at_next) = if let Some(next) = next {
            // closed vrp
            if actor.detail.time.end < next.place.time.start {
                return ConstraintViolation::fail(self.code);
            }
            (
                next.place.location,
                *route_ctx.state().get_activity_state(LATEST_ARRIVAL_KEY, next).unwrap_or(&next.place.time.end),
            )
        } else {
            // open vrp
            (target.place.location, target.place.time.end.min(actor.detail.time.end))
        };

        let arr_time_at_next = departure
            + self.transport.duration(route, prev.place.location, next_act_location, TravelTime::Departure(departure));

        if arr_time_at_next > latest_arr_time_at_next {
            return ConstraintViolation::fail(self.code);
        }
        if target.place.time.start > latest_arr_time_at_next {
            return ConstraintViolation::skip(self.code);
        }

        let arr_time_at_target = departure
            + self.transport.duration(
                route,
                prev.place.location,
                target.place.location,
                TravelTime::Departure(departure),
            );

        let latest_departure_at_target = latest_arr_time_at_next
            - self.transport.duration(
                route,
                target.place.location,
                next_act_location,
                TravelTime::Arrival(latest_arr_time_at_next),
            );

        let latest_arr_time_at_target =
            target.place.time.end.min(self.activity.estimate_arrival(route, target, latest_departure_at_target));

        if arr_time_at_target > latest_arr_time_at_target {
            return ConstraintViolation::skip(self.code);
        }

        if next.is_none() {
            return ConstraintViolation::success();
        }

        let end_time_at_target = self.activity.estimate_departure(route, target, arr_time_at_target);

        let arr_time_at_next = end_time_at_target
            + self.transport.duration(
                route,
                target.place.location,
                next_act_location,
                TravelTime::Departure(end_time_at_target),
            );

        if arr_time_at_next > latest_arr_time_at_next {
            ConstraintViolation::skip(self.code)
        } else {
            ConstraintViolation::success()
        }
    }
}

impl FeatureConstraint for TransportConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_job(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        // NOTE we don't change temporal parameters here, it is responsibility of the caller
        Ok(source)
    }
}

struct TransportObjective {
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    fitness_fn: Box<dyn Fn(&InsertionContext) -> f64 + Send + Sync>,
}

impl TransportObjective {
    fn estimate_route(&self, route_ctx: &RouteContext) -> f64 {
        if route_ctx.route().tour.has_jobs() {
            0.
        } else {
            route_ctx.route().actor.driver.costs.fixed + route_ctx.route().actor.vehicle.costs.fixed
        }
    }

    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> f64 {
        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let (tp_cost_left, act_cost_left, dep_time_left) =
            self.analyze_route_leg(route_ctx, prev, target, prev.schedule.departure);

        let (tp_cost_right, act_cost_right, dep_time_right) = if let Some(next) = next {
            self.analyze_route_leg(route_ctx, target, next, dep_time_left)
        } else {
            (0., 0., 0.)
        };

        let new_costs = tp_cost_left + tp_cost_right + act_cost_left + act_cost_right;

        // no jobs yet or open vrp.
        if !route_ctx.route().tour.has_jobs() || next.is_none() {
            return new_costs;
        }

        let next = next.unwrap();
        let waiting_time = *route_ctx.state().get_activity_state(WAITING_KEY, next).unwrap_or(&0_f64);

        let (tp_cost_old, act_cost_old, dep_time_old) =
            self.analyze_route_leg(route_ctx, prev, next, prev.schedule.departure);

        let waiting_cost = waiting_time.min(0.0_f64.max(dep_time_right - dep_time_old))
            * route_ctx.route().actor.vehicle.costs.per_waiting_time;

        let old_costs = tp_cost_old + act_cost_old + waiting_cost;

        new_costs - old_costs
    }

    fn analyze_route_leg(
        &self,
        route_ctx: &RouteContext,
        start: &Activity,
        end: &Activity,
        time: Timestamp,
    ) -> (Cost, Cost, Timestamp) {
        let route = route_ctx.route();

        let arrival = time
            + self.transport.duration(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let departure = self.activity.estimate_departure(route, end, arrival);

        let transport_cost =
            self.transport.cost(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let activity_cost = self.activity.cost(route, end, arrival);

        (transport_cost, activity_cost, departure)
    }
}

impl Objective for TransportObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        self.fitness_fn.deref()(solution)
    }
}

impl FeatureObjective for TransportObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => self.estimate_route(route_ctx),
            MoveContext::Activity { route_ctx, activity_ctx } => self.estimate_activity(route_ctx, activity_ctx),
        }
    }
}

struct TransportState {
    schedule_state_keys: ScheduleStateKeys,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    all_state_keys: Vec<StateKey>,
}

impl TransportState {
    fn new(transport: Arc<dyn TransportCost + Send + Sync>, activity: Arc<dyn ActivityCost + Send + Sync>) -> Self {
        let schedule_state_keys = ScheduleStateKeys::default();
        let all_state_keys = vec![
            schedule_state_keys.waiting_time,
            schedule_state_keys.latest_arrival,
            schedule_state_keys.total_duration,
            schedule_state_keys.total_distance,
        ];

        Self { schedule_state_keys: ScheduleStateKeys::default(), transport, activity, all_state_keys }
    }
}

impl FeatureState for TransportState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        update_route_schedule(route_ctx, self.activity.as_ref(), self.transport.as_ref(), &self.schedule_state_keys);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            update_route_schedule(
                route_ctx,
                self.activity.as_ref(),
                self.transport.as_ref(),
                &self.schedule_state_keys,
            );
        })
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.all_state_keys.iter()
    }
}

impl Default for ScheduleStateKeys {
    fn default() -> Self {
        Self {
            latest_arrival: LATEST_ARRIVAL_KEY,
            waiting_time: WAITING_KEY,
            total_distance: TOTAL_DISTANCE_KEY,
            total_duration: TOTAL_DURATION_KEY,
        }
    }
}
