#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/transport_test.rs"]
mod transport_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use crate::construction::OP_START_MSG;
use crate::models::common::{Cost, Distance, Duration, Profile, Timestamp};
use crate::models::problem::{ActivityCost, Actor, Job, Single, TransportCost};
use crate::models::solution::Activity;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

// TODO revise rescheduling once routing is sensible to departure time

pub type TravelLimitFunc = Arc<dyn Fn(&Actor) -> (Option<Distance>, Option<Duration>) + Send + Sync>;

/// A module which checks whether vehicle can serve activity taking into account their time windows
/// and traveling constraints. Also it is responsible for transport cost calculations.
pub struct TransportConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    limit_func: TravelLimitFunc,
}

impl ConstraintModule for TransportConstraintModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, route_ctx: &mut RouteContext, _job: &Job) {
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.update_route_schedules(ctx);
        self.update_route_states(ctx);
        // NOTE Rescheduling during the insertion process makes sense only if the traveling limit
        // is set (for duration limit, not for distance).
        if has_travel_limits(&self.limit_func, ctx) {
            self.reschedule_departure(ctx)
        }
        self.update_statistics(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        ctx.routes.iter_mut().for_each(|route_ctx| {
            self.update_route_schedules(route_ctx);
            self.update_route_states(route_ctx);
            self.reschedule_departure(route_ctx);
            self.update_statistics(route_ctx);
        })
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl TransportConstraintModule {
    pub fn new(
        activity: Arc<dyn ActivityCost + Send + Sync>,
        transport: Arc<dyn TransportCost + Send + Sync>,
        limit_func: TravelLimitFunc,
        time_window_code: i32,
        distance_code: i32,
        duration_code: i32,
    ) -> Self {
        Self {
            state_keys: vec![LATEST_ARRIVAL_KEY, WAITING_KEY],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(TimeHardRouteConstraint { code: time_window_code })),
                ConstraintVariant::SoftRoute(Arc::new(RouteCostSoftRouteConstraint {})),
                ConstraintVariant::HardActivity(Arc::new(TimeHardActivityConstraint {
                    code: time_window_code,
                    transport: transport.clone(),
                    activity: activity.clone(),
                })),
                ConstraintVariant::HardActivity(Arc::new(TravelHardActivityConstraint {
                    limit_func: limit_func.clone(),
                    distance_code,
                    duration_code,
                    transport: transport.clone(),
                })),
                ConstraintVariant::SoftActivity(Arc::new(CostSoftActivityConstraint {
                    transport: transport.clone(),
                    activity: activity.clone(),
                })),
            ],
            activity,
            transport,
            limit_func,
        }
    }

    fn update_route_schedules(&self, ctx: &mut RouteContext) {
        let (init, actor) = {
            let start = ctx.route.tour.start().unwrap();
            ((start.place.location, start.schedule.departure), ctx.route.actor.clone())
        };

        ctx.route_mut().tour.all_activities_mut().skip(1).fold(init, |(loc, dep), a| {
            a.schedule.arrival = dep + self.transport.duration(actor.vehicle.profile, loc, a.place.location, dep);
            a.schedule.departure = a.schedule.arrival.max(a.place.time.start)
                + self.activity.duration(actor.as_ref(), a.deref(), a.schedule.arrival);

            (a.place.location, a.schedule.departure)
        });
    }

    fn update_route_states(&self, ctx: &mut RouteContext) {
        // update latest arrival and waiting states of non-terminate (jobs) activities
        let actor = ctx.route.actor.clone();
        let init = (
            actor.detail.time.end,
            actor.detail.end.unwrap_or_else(|| actor.detail.start.unwrap_or_else(|| panic!(OP_START_MSG))),
            0_f64,
        );

        let (route, state) = ctx.as_mut();

        route.tour.all_activities().rev().fold(init, |acc, act| {
            if act.job.is_none() {
                return acc;
            }

            let (end_time, prev_loc, waiting) = acc;
            let potential_latest = end_time
                - self.transport.duration(actor.vehicle.profile, act.place.location, prev_loc, end_time)
                - self.activity.duration(actor.as_ref(), act.deref(), end_time);

            let latest_arrival_time = act.place.time.end.min(potential_latest);
            let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0_f64);

            state.put_activity_state(LATEST_ARRIVAL_KEY, &act, latest_arrival_time);
            state.put_activity_state(WAITING_KEY, &act, future_waiting);

            (latest_arrival_time, act.place.location, future_waiting)
        });
    }

    fn reschedule_departure(&self, ctx: &mut RouteContext) {
        if let Some((last_departure_time, new_departure_time)) = self.analyze_departures(ctx) {
            if new_departure_time > last_departure_time {
                let mut start = ctx.route_mut().tour.get_mut(0).unwrap();
                start.schedule.departure = new_departure_time;
                self.update_route_schedules(ctx);
                self.update_route_states(ctx);
            }
        }
    }

    fn analyze_departures(&self, ctx: &RouteContext) -> Option<(Timestamp, Timestamp)> {
        if let Some(first) = ctx.route.tour.get(1) {
            let start = ctx.route.tour.start().unwrap();
            let last_departure_time = start.schedule.departure;
            let start_to_first = self.transport.duration(
                ctx.route.actor.vehicle.profile,
                start.place.location,
                first.place.location,
                last_departure_time,
            );
            let new_departure_time = last_departure_time.max(first.place.time.start - start_to_first);
            return Some((last_departure_time, new_departure_time));
        }
        None
    }

    fn update_statistics(&self, ctx: &mut RouteContext) {
        let start = ctx.route.tour.start().unwrap();
        let end = ctx.route.tour.end().unwrap();

        let total_dur = end.schedule.departure - start.schedule.departure;

        let init = (start.place.location, start.schedule.departure, Distance::default());
        let (_, _, total_dist) = ctx.route.tour.all_activities().skip(1).fold(init, |(loc, dep, total_dist), a| {
            let total_dist =
                total_dist + self.transport.distance(ctx.route.actor.vehicle.profile, loc, a.place.location, dep);

            (a.place.location, a.schedule.departure, total_dist)
        });

        ctx.state_mut().put_route_state(TOTAL_DISTANCE_KEY, total_dist);
        ctx.state_mut().put_route_state(TOTAL_DURATION_KEY, total_dur);
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

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let departure = prev.schedule.departure;
        let profile = actor.vehicle.profile;

        if actor.detail.time.end < prev.place.time.start
            || actor.detail.time.end < target.place.time.start
            || next.map_or(false, |next| actor.detail.time.end < next.place.time.start)
        {
            return fail(self.code);
        }

        let (next_act_location, latest_arr_time_at_next_act) = if let Some(next) = next {
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

        let arr_time_at_next =
            departure + self.transport.duration(profile, prev.place.location, next_act_location, departure);

        if arr_time_at_next > latest_arr_time_at_next_act {
            return fail(self.code);
        }
        if target.place.time.start > latest_arr_time_at_next_act {
            return stop(self.code);
        }

        let arr_time_at_target_act =
            departure + self.transport.duration(profile, prev.place.location, target.place.location, departure);

        let end_time_at_new_act = arr_time_at_target_act.max(target.place.time.start)
            + self.activity.duration(actor, target.deref(), arr_time_at_target_act);

        let latest_arr_time_at_new_act = target.place.time.end.min(
            latest_arr_time_at_next_act
                - self.transport.duration(
                    profile,
                    target.place.location,
                    next_act_location,
                    latest_arr_time_at_next_act,
                )
                + self.activity.duration(actor, target.deref(), arr_time_at_target_act),
        );

        if arr_time_at_target_act > latest_arr_time_at_new_act {
            return stop(self.code);
        }

        if next.is_none() {
            return success();
        }

        let arr_time_at_next_act = end_time_at_new_act
            + self.transport.duration(profile, target.place.location, next_act_location, end_time_at_new_act);

        if arr_time_at_next_act > latest_arr_time_at_next_act {
            stop(self.code)
        } else {
            success()
        }
    }
}

/// A hard activity constraint which allows to limit actor's traveling distance and time.
struct TravelHardActivityConstraint {
    limit_func: TravelLimitFunc,
    distance_code: i32,
    duration_code: i32,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl HardActivityConstraint for TravelHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let limit = (self.limit_func)(&route_ctx.route.actor);
        if limit.0.is_some() || limit.1.is_some() {
            let (change_distance, change_duration) = self.calculate_travel(route_ctx, activity_ctx);

            let curr_dis = route_ctx.state.get_route_state(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
            let curr_dur = route_ctx.state.get_route_state(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

            let total_distance = curr_dis + change_distance;
            let total_duration = curr_dur + change_duration;

            match limit {
                (Some(max_distance), _) if max_distance < total_distance => stop(self.distance_code),
                (_, Some(max_duration)) if max_duration < total_duration => stop(self.duration_code),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl TravelHardActivityConstraint {
    fn calculate_travel(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        let actor = &route_ctx.route.actor;
        let profile = actor.vehicle.profile;

        let prev = activity_ctx.prev;
        let tar = activity_ctx.target;
        let next = activity_ctx.next;

        let prev_dep = prev.schedule.departure;

        let (prev_to_tar_dis, prev_to_tar_dur) = self.calculate_leg_travel_info(profile, prev, tar, prev_dep);
        if next.is_none() {
            return (prev_to_tar_dis, prev_to_tar_dur);
        }

        let next = next.unwrap();
        let tar_dep = prev_dep + prev_to_tar_dur;

        let (prev_to_next_dis, prev_to_next_dur) = self.calculate_leg_travel_info(profile, prev, next, prev_dep);
        let (tar_to_next_dis, tar_to_next_dur) = self.calculate_leg_travel_info(profile, tar, next, tar_dep);

        (prev_to_tar_dis + tar_to_next_dis - prev_to_next_dis, prev_to_tar_dur + tar_to_next_dur - prev_to_next_dur)
    }

    fn calculate_leg_travel_info(
        &self,
        profile: Profile,
        first: &Activity,
        second: &Activity,
        departure: Timestamp,
    ) -> (Distance, Duration) {
        let first_to_second_dis =
            self.transport.distance(profile, first.place.location, second.place.location, departure);
        let first_to_second_dur =
            self.transport.duration(profile, first.place.location, second.place.location, departure);

        let second_arr = departure + first_to_second_dur;
        let second_wait = (second.place.time.start - second_arr).max(0.);
        let second_dep = second_arr + second_wait + second.place.duration;

        (first_to_second_dis, second_dep - departure)
    }
}

fn has_travel_limits(limit_func: &TravelLimitFunc, route_ctx: &RouteContext) -> bool {
    match (limit_func)(&route_ctx.route.actor) {
        (Some(_), _) => true,
        (_, Some(_)) => true,
        _ => false,
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
        actor: &Actor,
        start: &Activity,
        end: &Activity,
        time: Timestamp,
    ) -> (Cost, Cost, Timestamp) {
        let arrival =
            time + self.transport.duration(actor.vehicle.profile, start.place.location, end.place.location, time);
        let departure = arrival.max(end.place.time.start) + self.activity.duration(actor, end, arrival);

        let transport_cost = self.transport.cost(actor, start.place.location, end.place.location, time);
        let activity_cost = self.activity.cost(actor, end, arrival);

        (transport_cost, activity_cost, departure)
    }
}

impl SoftActivityConstraint for CostSoftActivityConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> f64 {
        let actor = route_ctx.route.actor.as_ref();

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let (tp_cost_left, act_cost_left, dep_time_left) =
            self.analyze_route_leg(actor, prev, target, prev.schedule.departure);

        let (tp_cost_right, act_cost_right, dep_time_right) = if let Some(next) = next {
            self.analyze_route_leg(actor, target, next, dep_time_left)
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
            self.analyze_route_leg(actor, prev, next, prev.schedule.departure);

        let waiting_cost =
            waiting_time.min(0_f64.max(dep_time_right - dep_time_old)) * actor.vehicle.costs.per_waiting_time;

        let old_costs = tp_cost_old + act_cost_old + waiting_cost;

        new_costs - old_costs
    }
}

fn fail(code: i32) -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code, stopped: true })
}

fn stop(code: i32) -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code, stopped: false })
}

fn success() -> Option<ActivityConstraintViolation> {
    None
}
