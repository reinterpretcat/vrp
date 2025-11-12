use crate::construction::heuristics::{RouteContext, RouteState};
use crate::models::OP_START_MSG;
use crate::models::common::{Distance, Duration, Schedule, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use rosomaxa::prelude::Float;

custom_activity_state!(pub(crate) LatestArrival typeof Timestamp);
custom_activity_state!(pub(crate) WaitingTime typeof Timestamp);
custom_tour_state!(pub TotalDistance typeof Distance);
custom_tour_state!(pub TotalDuration typeof Duration);
custom_tour_state!(pub(crate) LimitDuration typeof Duration);

/// Updates route schedule data.
pub fn update_route_schedule(route_ctx: &mut RouteContext, activity: &dyn ActivityCost, transport: &dyn TransportCost) {
    update_schedules(route_ctx, activity, transport);
    update_states(route_ctx, activity, transport);
    update_statistics(route_ctx, transport);
}

/// Updates route departure to the new one.
pub fn update_route_departure(
    route_ctx: &mut RouteContext,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    new_departure_time: Timestamp,
) {
    let start = route_ctx.route_mut().tour.get_mut(0).unwrap();
    start.schedule.departure = new_departure_time;

    update_route_schedule(route_ctx, activity, transport);
}

fn update_schedules(route_ctx: &mut RouteContext, activity: &dyn ActivityCost, transport: &dyn TransportCost) {
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

fn update_states(route_ctx: &mut RouteContext, activity: &dyn ActivityCost, transport: &dyn TransportCost) {
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
        Float::default(),
    );

    let route = route_ctx.route();
    let mut latest_arrivals = Vec::with_capacity(route.tour.total());
    let mut waiting_times = Vec::with_capacity(route.tour.total());

    route.tour.all_activities().rev().fold(init, |acc, act| {
        if act.job.is_none() {
            latest_arrivals.push(Default::default());
            waiting_times.push(Default::default());
            return acc;
        }

        let (end_time, prev_loc, waiting) = acc;
        let latest_arrival_time = if end_time == Float::MAX {
            act.place.time.end
        } else {
            let latest_departure =
                end_time - transport.duration(route, act.place.location, prev_loc, TravelTime::Arrival(end_time));
            activity.estimate_arrival(route, act, latest_departure)
        };
        let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0.);

        latest_arrivals.push(latest_arrival_time);
        waiting_times.push(future_waiting);

        (latest_arrival_time, act.place.location, future_waiting)
    });

    latest_arrivals.reverse();
    waiting_times.reverse();

    // NOTE: pop out state for arrival
    if route.tour.end().is_some_and(|end| end.job.is_none()) {
        latest_arrivals.pop();
        waiting_times.pop();
    }

    route_ctx.state_mut().set_latest_arrival_states(latest_arrivals);
    route_ctx.state_mut().set_waiting_time_states(waiting_times);
}

fn update_statistics(route_ctx: &mut RouteContext, transport: &dyn TransportCost) {
    let (route, state) = route_ctx.as_mut();

    let start = route.tour.start().unwrap();
    let end = route.tour.end().unwrap();
    let total_dur = end.schedule.departure - start.schedule.departure;

    let init = (start.place.location, start.schedule.departure, Distance::default());
    let (_, _, total_dist) = route.tour.all_activities().skip(1).fold(init, |(loc, dep, total_dist), a| {
        let total_dist = total_dist + transport.distance(route, loc, a.place.location, TravelTime::Departure(dep));

        (a.place.location, a.schedule.departure, total_dist)
    });

    state.set_total_distance(total_dist);
    state.set_total_duration(total_dur);
}
