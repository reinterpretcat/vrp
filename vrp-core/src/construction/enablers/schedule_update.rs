use crate::construction::heuristics::{RouteContext, StateKey, StateKeyRegistry};
use crate::models::common::{Distance, Schedule, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use crate::models::OP_START_MSG;

/// Contains state keys ids used by route schedule updating logic.
#[derive(Clone, Debug)]
pub struct ScheduleKeys {
    /// Latest arrival state key.
    pub latest_arrival: StateKey,
    /// Waiting time state key.
    pub waiting_time: StateKey,
    /// Total distance state key.
    pub total_distance: StateKey,
    /// Total duration state key.
    pub total_duration: StateKey,
    /// Limit duration state key.
    pub limit_duration: StateKey,
}

impl From<&mut StateKeyRegistry> for ScheduleKeys {
    fn from(state_registry: &mut StateKeyRegistry) -> Self {
        Self {
            latest_arrival: state_registry.next_key(),
            waiting_time: state_registry.next_key(),
            total_distance: state_registry.next_key(),
            total_duration: state_registry.next_key(),
            limit_duration: state_registry.next_key(),
        }
    }
}

/// Updates route schedule data.
pub fn update_route_schedule(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleKeys,
) {
    update_schedules(route_ctx, activity, transport);
    update_states(route_ctx, activity, transport, state_keys);
    update_statistics(route_ctx, transport, state_keys);
}

/// Updates route departure to the new one.
pub fn update_route_departure(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    new_departure_time: Timestamp,
    state_keys: &ScheduleKeys,
) {
    let start = route_ctx.route_mut().tour.get_mut(0).unwrap();
    start.schedule.departure = new_departure_time;

    update_route_schedule(route_ctx, activity, transport, state_keys);
}

fn update_schedules(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
) {
    let init = {
        let start = route_ctx.route().tour.start().unwrap();
        (start.place.location, start.schedule.departure)
    };

    (1..route_ctx.route().tour.total()).fold(init, |(loc, dep), activity_idx| {
        let (location, arrival, departure) = {
            let a = route_ctx.route().tour.get(activity_idx).unwrap();
            let location = a.place.location;
            let arrival = dep + transport.duration(route_ctx.route(), loc, location, TravelTime::Departure(dep));
            let departure = activity.estimate_departure(route_ctx.route(), a, arrival);

            (location, arrival, departure)
        };

        route_ctx.route_mut().tour.get_mut(activity_idx).unwrap().schedule = Schedule::new(arrival, departure);

        (location, departure)
    });
}

fn update_states(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleKeys,
) {
    // update latest arrival and waiting states of non-terminate (jobs) activities
    let actor = route_ctx.route().actor.clone();
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

    let (route, state) = route_ctx.as_mut();

    route.tour.all_activities().enumerate().rev().fold(init, |acc, (activity_idx, act)| {
        if act.job.is_none() {
            return acc;
        }

        let (end_time, prev_loc, waiting) = acc;
        let latest_arrival_time = if end_time == f64::MAX {
            act.place.time.end
        } else {
            let latest_departure =
                end_time - transport.duration(route, act.place.location, prev_loc, TravelTime::Arrival(end_time));
            activity.estimate_arrival(route, act, latest_departure)
        };
        let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0.);

        state.put_activity_state(state_keys.latest_arrival, activity_idx, latest_arrival_time);
        state.put_activity_state(state_keys.waiting_time, activity_idx, future_waiting);

        (latest_arrival_time, act.place.location, future_waiting)
    });
}

fn update_statistics(
    route_ctx: &mut RouteContext,
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleKeys,
) {
    let (route, state) = route_ctx.as_mut();

    let start = route.tour.start().unwrap();
    let end = route.tour.end().unwrap();
    let total_dur = end.schedule.departure - start.schedule.departure;

    let init = (start.place.location, start.schedule.departure, Distance::default());
    let (_, _, total_dist) =
        route.tour.all_activities().enumerate().skip(1).fold(init, |(loc, dep, total_dist), (a_idx, a)| {
            let total_dist = total_dist + transport.distance(route, loc, a.place.location, TravelTime::Departure(dep));
            let total_dur = a.schedule.departure - start.schedule.departure;

            state.put_activity_state(state_keys.total_distance, a_idx, total_dist);
            state.put_activity_state(state_keys.total_duration, a_idx, total_dur);

            (a.place.location, a.schedule.departure, total_dist)
        });

    state.put_route_state(state_keys.total_distance, total_dist);
    state.put_route_state(state_keys.total_duration, total_dur);
}
