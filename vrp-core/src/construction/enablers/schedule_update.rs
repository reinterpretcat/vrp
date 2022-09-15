use crate::construction::heuristics::RouteContext;
use crate::models::common::StateKey;
use crate::models::common::{Distance, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use crate::models::OP_START_MSG;

/// Contains state keys ids used by route schedule updating logic.
pub struct ScheduleStateKeys {
    /// Latest arrival state key.
    pub latest_arrival: StateKey,
    /// Waiting time state key.
    pub waiting_time: StateKey,
    /// Total distance state key.
    pub total_distance: StateKey,
    /// Total duration state key.
    pub total_duration: StateKey,
}

/// Updates route schedule data.
pub fn update_route_schedule(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleStateKeys,
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
    state_keys: &ScheduleStateKeys,
) {
    let mut start = route_ctx.route_mut().tour.get_mut(0).unwrap();
    start.schedule.departure = new_departure_time;

    update_route_schedule(route_ctx, activity, transport, state_keys);
}

fn update_schedules(
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

fn update_states(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleStateKeys,
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

        state.put_activity_state(state_keys.latest_arrival, act, latest_arrival_time);
        state.put_activity_state(state_keys.waiting_time, act, future_waiting);

        (latest_arrival_time, act.place.location, future_waiting)
    });
}

fn update_statistics(
    route_ctx: &mut RouteContext,
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleStateKeys,
) {
    let route = route_ctx.route.clone();
    let start = route.tour.start().unwrap();
    let end = route.tour.end().unwrap();

    let total_dur = end.schedule.departure - start.schedule.departure;

    let init = (start.place.location, start.schedule.departure, Distance::default());
    let (_, _, total_dist) = route.tour.all_activities().skip(1).fold(init, |(loc, dep, total_dist), a| {
        let total_dist =
            total_dist + transport.distance(route.as_ref(), loc, a.place.location, TravelTime::Departure(dep));
        let total_dur = a.schedule.departure - start.schedule.departure;

        route_ctx.state_mut().put_activity_state(state_keys.total_distance, a, total_dist);
        route_ctx.state_mut().put_activity_state(state_keys.total_duration, a, total_dur);

        (a.place.location, a.schedule.departure, total_dist)
    });

    route_ctx.state_mut().put_route_state(state_keys.total_distance, total_dist);
    route_ctx.state_mut().put_route_state(state_keys.total_duration, total_dur);
}
