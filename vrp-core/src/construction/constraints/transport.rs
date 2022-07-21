#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/transport_test.rs"]
mod transport_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::{Cost, Distance, Timestamp};
use crate::models::problem::{ActivityCost, Job, Single, TransportCost, TravelLimits, TravelTime};
use crate::models::solution::Activity;
use crate::models::OP_START_MSG;
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;
use std::slice::Iter;
use std::sync::Arc;

// TODO revise rescheduling once routing is sensible to departure time

/// A module which checks whether vehicle can serve activity taking into account their time windows
/// and traveling constraints. Also it is responsible for transport cost calculations.
pub struct TransportConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl ConstraintModule for TransportConstraintModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _job: &Job) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let activity = self.activity.as_ref();
        let transport = self.transport.as_ref();

        Self::update_route_schedules(ctx, activity, transport);
        Self::update_route_states(ctx, activity, transport);
        // NOTE Rescheduling during the insertion process makes sense only if the traveling limit
        // is set (for duration limit, not for distance).
        if transport.limits().tour_duration(&ctx.route.actor).is_some() {
            Self::advance_departure_time(ctx, activity, transport, false);
        }

        Self::update_statistics(ctx, transport);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            let activity = self.activity.as_ref();
            let transport = self.transport.as_ref();

            Self::update_route_schedules(route_ctx, activity, transport);
            Self::update_route_states(route_ctx, activity, transport);
            Self::update_statistics(route_ctx, transport);
        })
    }

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        // NOTE we don't change temporal parameters here, it is responsibility of the caller
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl TransportConstraintModule {
    /// Creates a new instance of `TransportConstraintModule`.
    pub fn new(
        transport: Arc<dyn TransportCost + Send + Sync>,
        activity: Arc<dyn ActivityCost + Send + Sync>,
        time_window_code: i32,
    ) -> Self {
        Self {
            state_keys: vec![LATEST_ARRIVAL_KEY, WAITING_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(TimeHardRouteConstraint { code: time_window_code })),
                ConstraintVariant::SoftRoute(Arc::new(RouteCostSoftRouteConstraint {})),
                ConstraintVariant::HardActivity(Arc::new(TimeHardActivityConstraint {
                    code: time_window_code,
                    activity: activity.clone(),
                    transport: transport.clone(),
                })),
                ConstraintVariant::SoftActivity(Arc::new(CostSoftActivityConstraint {
                    transport: transport.clone(),
                    activity: activity.clone(),
                })),
            ],
            activity,
            transport,
        }
    }

    fn update_route_schedules(
        route_ctx: &mut RouteContext,
        activity: &(dyn ActivityCost + Send + Sync),
        transport: &(dyn TransportCost + Send + Sync),
    ) {
        let init = {
            let start = route_ctx.route.tour.start().unwrap();
            (start.place.location, start.schedule.departure)
        };

        let route = route_ctx.route.clone();

        route_ctx.route_mut().tour.all_activities_mut().skip(1).fold(init, |(loc, dep), a| {
            a.schedule.arrival = dep + transport.duration(&route, loc, a.place.location, TravelTime::Departure(dep));
            a.schedule.departure = activity.estimate_departure(&route, a, a.schedule.arrival);

            (a.place.location, a.schedule.departure)
        });
    }

    fn update_route_states(
        route_ctx: &mut RouteContext,
        activity: &(dyn ActivityCost + Send + Sync),
        transport: &(dyn TransportCost + Send + Sync),
    ) {
        // update latest arrival and waiting states of non-terminate (jobs) activities
        let actor = route_ctx.route.actor.clone();
        let init = (
            actor.detail.time.end,
            actor
                .detail
                .end
                .as_ref()
                .unwrap_or_else(|| actor.detail.start.as_ref().unwrap_or_else(|| panic!("{}", OP_START_MSG)))
                .location,
            0_f64,
        );

        let route = route_ctx.route.clone();
        let (route_mut, state) = route_ctx.as_mut();

        route_mut.tour.all_activities().rev().fold(init, |acc, act| {
            if act.job.is_none() {
                return acc;
            }

            let (end_time, prev_loc, waiting) = acc;
            let latest_departure =
                end_time - transport.duration(&route, act.place.location, prev_loc, TravelTime::Arrival(end_time));
            let latest_arrival_time = activity.estimate_arrival(&route, act, latest_departure);
            let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0.);

            state.put_activity_state(LATEST_ARRIVAL_KEY, act, latest_arrival_time);
            state.put_activity_state(WAITING_KEY, act, future_waiting);

            (latest_arrival_time, act.place.location, future_waiting)
        });
    }

    fn update_statistics(route_ctx: &mut RouteContext, transport: &(dyn TransportCost + Send + Sync)) {
        let route = route_ctx.route.clone();
        let start = route.tour.start().unwrap();
        let end = route.tour.end().unwrap();

        let total_dur = end.schedule.departure - start.schedule.departure;

        let init = (start.place.location, start.schedule.departure, Distance::default());
        let (_, _, total_dist) = route.tour.all_activities().skip(1).fold(init, |(loc, dep, total_dist), a| {
            let total_dist =
                total_dist + transport.distance(route.as_ref(), loc, a.place.location, TravelTime::Departure(dep));
            let total_dur = a.schedule.departure - start.schedule.departure;

            route_ctx.state_mut().put_activity_state(TOTAL_DISTANCE_KEY, a, total_dist);
            route_ctx.state_mut().put_activity_state(TOTAL_DURATION_KEY, a, total_dur);

            (a.place.location, a.schedule.departure, total_dist)
        });

        route_ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, total_dist);
        route_ctx.state_mut().put_route_state(TOTAL_DURATION_KEY, total_dur);
    }

    /// Tries to move forward route's departure time.
    pub(crate) fn advance_departure_time(
        route_ctx: &mut RouteContext,
        activity: &(dyn ActivityCost + Send + Sync),
        transport: &(dyn TransportCost + Send + Sync),
        consider_whole_tour: bool,
    ) {
        let new_departure_time = try_advance_departure_time(route_ctx, transport, consider_whole_tour);
        Self::try_update_route_departure(route_ctx, activity, transport, new_departure_time);
    }

    /// Tries to move backward route's departure time.
    pub(crate) fn recede_departure_time(
        route_ctx: &mut RouteContext,
        activity: &(dyn ActivityCost + Send + Sync),
        transport: &(dyn TransportCost + Send + Sync),
    ) {
        let new_departure_time = try_recede_departure_time(route_ctx, transport.limits());
        Self::try_update_route_departure(route_ctx, activity, transport, new_departure_time);
    }

    fn try_update_route_departure(
        ctx: &mut RouteContext,
        activity: &(dyn ActivityCost + Send + Sync),
        transport: &(dyn TransportCost + Send + Sync),
        new_departure_time: Option<f64>,
    ) {
        if let Some(new_departure_time) = new_departure_time {
            let mut start = ctx.route_mut().tour.get_mut(0).unwrap();
            start.schedule.departure = new_departure_time;
            Self::update_route_schedules(ctx, activity, transport);
            Self::update_route_states(ctx, activity, transport);
        }
    }
}

struct TimeHardRouteConstraint {
    code: i32,
}

impl HardRouteConstraint for TimeHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        let date = ctx.route.tour.start().unwrap().schedule.departure;
        let check_single = |single: &Arc<Single>| {
            single
                .places
                .iter()
                .flat_map(|place| place.times.iter())
                .any(|time| time.intersects(date, &ctx.route.actor.detail.time))
        };

        let has_time_intersection = match job {
            Job::Single(single) => check_single(single),
            Job::Multi(multi) => multi.jobs.iter().all(check_single),
        };

        if has_time_intersection {
            None
        } else {
            Some(RouteConstraintViolation { code: self.code })
        }
    }
}

/// Checks time windows of actor and job.
struct TimeHardActivityConstraint {
    code: i32,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl HardActivityConstraint for TimeHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let actor = route_ctx.route.actor.as_ref();
        let route = route_ctx.route.as_ref();

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let departure = prev.schedule.departure;

        if actor.detail.time.end < prev.place.time.start
            || actor.detail.time.end < target.place.time.start
            || next.map_or(false, |next| actor.detail.time.end < next.place.time.start)
        {
            return fail(self.code);
        }

        let (next_act_location, latest_arr_time_at_next) = if let Some(next) = next {
            // closed vrp
            if actor.detail.time.end < next.place.time.start {
                return fail(self.code);
            }
            (
                next.place.location,
                *route_ctx.state.get_activity_state(LATEST_ARRIVAL_KEY, next).unwrap_or(&next.place.time.end),
            )
        } else {
            // open vrp
            (target.place.location, target.place.time.end.min(actor.detail.time.end))
        };

        let arr_time_at_next = departure
            + self.transport.duration(route, prev.place.location, next_act_location, TravelTime::Departure(departure));

        if arr_time_at_next > latest_arr_time_at_next {
            return fail(self.code);
        }
        if target.place.time.start > latest_arr_time_at_next {
            return stop(self.code);
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
            return stop(self.code);
        }

        if next.is_none() {
            return success();
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
            stop(self.code)
        } else {
            success()
        }
    }
}

/// Applies fixed cost for actor usage.
struct RouteCostSoftRouteConstraint {}

impl SoftRouteConstraint for RouteCostSoftRouteConstraint {
    fn estimate_job(&self, _: &SolutionContext, ctx: &RouteContext, _job: &Job) -> f64 {
        if ctx.route.tour.job_count() == 0 {
            ctx.route.actor.driver.costs.fixed + ctx.route.actor.vehicle.costs.fixed
        } else {
            0.
        }
    }
}

/// Calculates transportation costs.
struct CostSoftActivityConstraint {
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl CostSoftActivityConstraint {
    fn analyze_route_leg(
        &self,
        route_ctx: &RouteContext,
        start: &Activity,
        end: &Activity,
        time: Timestamp,
    ) -> (Cost, Cost, Timestamp) {
        let route = route_ctx.route.as_ref();

        let arrival = time
            + self.transport.duration(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let departure = self.activity.estimate_departure(route, end, arrival);

        let transport_cost =
            self.transport.cost(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let activity_cost = self.activity.cost(route, end, arrival);

        (transport_cost, activity_cost, departure)
    }
}

impl SoftActivityConstraint for CostSoftActivityConstraint {
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
        if !route_ctx.route.tour.has_jobs() || next.is_none() {
            return new_costs;
        }

        let next = next.unwrap();
        let waiting_time = *route_ctx.state.get_activity_state(WAITING_KEY, next).unwrap_or(&0_f64);

        let (tp_cost_old, act_cost_old, dep_time_old) =
            self.analyze_route_leg(route_ctx, prev, next, prev.schedule.departure);

        let waiting_cost = waiting_time.min(0.0_f64.max(dep_time_right - dep_time_old))
            * route_ctx.route.actor.vehicle.costs.per_waiting_time;

        let old_costs = tp_cost_old + act_cost_old + waiting_cost;

        new_costs - old_costs
    }
}

fn try_advance_departure_time(
    route_ctx: &RouteContext,
    transport: &(dyn TransportCost + Send + Sync),
    optimize_whole_tour: bool,
) -> Option<Timestamp> {
    let route = route_ctx.route.as_ref();

    let first = route.tour.get(1)?;
    let start = route.tour.start()?;

    let latest_allowed_departure = route.actor.detail.start.as_ref().and_then(|s| s.time.latest).unwrap_or(f64::MAX);
    let last_departure_time = start.schedule.departure;

    let new_departure_time = if optimize_whole_tour {
        let (total_waiting_time, max_shift) =
            route.tour.all_activities().rev().fold((0., f64::MAX), |(total_waiting_time, max_shift), activity| {
                let waiting_time = (activity.place.time.start - activity.schedule.arrival).max(0.);
                let remaining_time = (activity.place.time.end - activity.schedule.arrival - waiting_time).max(0.);

                (total_waiting_time + waiting_time, waiting_time + remaining_time.min(max_shift))
            });
        let departure_shift = total_waiting_time.min(max_shift);

        (start.schedule.departure + departure_shift).min(latest_allowed_departure)
    } else {
        let start_to_first = transport.duration(
            route,
            start.place.location,
            first.place.location,
            TravelTime::Departure(last_departure_time),
        );

        last_departure_time.max(first.place.time.start - start_to_first).min(latest_allowed_departure)
    };

    if new_departure_time > last_departure_time {
        Some(new_departure_time)
    } else {
        None
    }
}

fn try_recede_departure_time(
    route_ctx: &RouteContext,
    travel_limits: &(dyn TravelLimits + Send + Sync),
) -> Option<Timestamp> {
    let first = route_ctx.route.tour.get(1)?;
    let start = route_ctx.route.tour.start()?;

    let max_change = *route_ctx.state.get_activity_state::<f64>(LATEST_ARRIVAL_KEY, first)? - first.schedule.arrival;

    let earliest_allowed_departure =
        route_ctx.route.actor.detail.start.as_ref().and_then(|s| s.time.earliest).unwrap_or(start.place.time.start);

    let max_change = (start.schedule.departure - earliest_allowed_departure).min(max_change);

    let max_change = route_ctx
        .state
        .get_route_state::<f64>(TOTAL_DURATION_KEY)
        .zip(travel_limits.tour_duration(route_ctx.route.actor.as_ref()))
        .map(|(&total, limit)| (limit - total).min(max_change))
        .unwrap_or(max_change);

    match compare_floats(max_change, 0.) {
        Ordering::Greater => Some(start.schedule.departure - max_change),
        _ => None,
    }
}
