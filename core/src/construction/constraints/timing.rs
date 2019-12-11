#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/timing_test.rs"]
mod timing_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{ActivityCost, Actor, TransportCost};
use crate::models::solution::Activity;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

const LATEST_ARRIVAL_KEY: i32 = 1;
const WAITING_KEY: i32 = 2;
const OP_START_MSG: &str = "Optional start is not yet implemented.";

/// Checks whether vehicle can serve activity taking into account their time windows.
/// TODO add extra check that job's and actor's TWs have intersection (hard route constraint).
pub struct TimingConstraintModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl ConstraintModule for TimingConstraintModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        self.update_route_schedules(ctx);
        self.update_route_states(ctx);
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // NOTE revise this once routing is sensible to departure time reschedule departure and
        // arrivals if arriving earlier to the first activity do it only in implicit end of algorithm
        if ctx.required.is_empty() {
            ctx.routes.iter_mut().for_each(|rc| {
                self.accept_route_state(rc);
                self.reschedule_departure(rc)
            })
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl TimingConstraintModule {
    pub fn new(
        activity: Arc<dyn ActivityCost + Send + Sync>,
        transport: Arc<dyn TransportCost + Send + Sync>,
        code: i32,
    ) -> Self {
        Self {
            state_keys: vec![LATEST_ARRIVAL_KEY, WAITING_KEY],
            constraints: vec![
                ConstraintVariant::HardActivity(Arc::new(TimeHardActivityConstraint {
                    code,
                    transport: transport.clone(),
                    activity: activity.clone(),
                })),
                ConstraintVariant::SoftActivity(Arc::new(TimeSoftActivityConstraint {
                    transport: transport.clone(),
                    activity: activity.clone(),
                })),
            ],
            activity,
            transport,
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
            0f64,
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
            let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0f64);

            state.put_activity_state(LATEST_ARRIVAL_KEY, &act, latest_arrival_time);
            state.put_activity_state(WAITING_KEY, &act, future_waiting);

            (latest_arrival_time, act.place.location, future_waiting)
        });
    }

    fn reschedule_departure(&self, ctx: &mut RouteContext) {
        if let Some((new_departure_time, earliest_departure_time)) = self.analyze_departures(ctx) {
            if new_departure_time > earliest_departure_time {
                let mut start = ctx.route_mut().tour.get_mut(0).unwrap();
                start.schedule.departure = new_departure_time;
                self.accept_route_state(ctx);
            }
        }
    }

    fn analyze_departures(&self, ctx: &RouteContext) -> Option<(Timestamp, Timestamp)> {
        if let Some(first) = ctx.route.tour.get(1) {
            let start = ctx.route.tour.start().unwrap();
            let earliest_departure_time = start.place.time.start;
            let start_to_first = self.transport.duration(
                ctx.route.actor.vehicle.profile,
                start.place.location,
                first.place.location,
                earliest_departure_time,
            );
            let new_departure_time = earliest_departure_time.max(first.place.time.start - start_to_first);
            return Some((new_departure_time, earliest_departure_time));
        }
        None
    }
}

struct TimeHardActivityConstraint {
    code: i32,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl TimeHardActivityConstraint {
    fn fail(&self) -> Option<ActivityConstraintViolation> {
        Some(ActivityConstraintViolation { code: self.code, stopped: true })
    }

    fn stop(&self) -> Option<ActivityConstraintViolation> {
        Some(ActivityConstraintViolation { code: self.code, stopped: false })
    }

    fn success(&self) -> Option<ActivityConstraintViolation> {
        None
    }
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
            return self.fail();
        }

        let (next_act_location, latest_arr_time_at_next_act) = if let Some(next) = next {
            // closed vrp
            if actor.detail.time.end < next.place.time.start {
                return self.fail();
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
            return self.fail();
        }
        if target.place.time.start > latest_arr_time_at_next_act {
            return self.stop();
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
            return self.stop();
        }

        if next.is_none() {
            return self.success();
        }

        let arr_time_at_next_act = end_time_at_new_act
            + self.transport.duration(profile, target.place.location, next_act_location, end_time_at_new_act);

        if arr_time_at_next_act > latest_arr_time_at_next_act {
            self.stop()
        } else {
            self.success()
        }
    }
}

struct TimeSoftActivityConstraint {
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
}

impl TimeSoftActivityConstraint {
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

impl SoftActivityConstraint for TimeSoftActivityConstraint {
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
        let waiting_time = *route_ctx.state.get_activity_state(WAITING_KEY, next).unwrap_or(&0.0f64);

        let (tp_cost_old, act_cost_old, dep_time_old) =
            self.analyze_route_leg(actor, prev, next, prev.schedule.departure);

        let waiting_cost =
            waiting_time.min(0f64.max(dep_time_right - dep_time_old)) * actor.vehicle.costs.per_waiting_time;

        let old_costs = tp_cost_old + act_cost_old + waiting_cost;

        new_costs - old_costs
    }
}
