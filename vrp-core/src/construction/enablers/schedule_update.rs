use crate::construction::heuristics::{RouteContext, RouteState};
use crate::models::OP_START_MSG;
use crate::models::common::{Distance, Duration, Schedule, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use rosomaxa::prelude::Float;
use rosomaxa::utils::UnwrapValue;

custom_tour_state!(pub TotalDistance typeof Distance);
custom_tour_state!(pub TotalDuration typeof Duration);
custom_tour_state!(pub(crate) LimitDuration typeof Duration);

#[derive(Clone, Copy, Default)]
pub(crate) struct ActivityScheduleState {
    pub latest_arrival: Timestamp,
    pub waiting_time: Timestamp,
}

custom_activity_state!(pub(crate) Schedule typeof ActivityScheduleState, setter(cfg(test)));

trait ScheduleStateAccess {
    fn prepare_schedule_states(&mut self, activity_count: usize) -> &mut Vec<ActivityScheduleState>;
}

impl ScheduleStateAccess for RouteState {
    fn prepare_schedule_states(&mut self, activity_count: usize) -> &mut Vec<ActivityScheduleState> {
        let states =
            self.get_or_init_exclusive_tour_state::<ScheduleActivityStateKey, Vec<ActivityScheduleState>>(|| {
                Vec::with_capacity(activity_count)
            });

        states.clear();
        states.reserve(activity_count);
        states
    }
}

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
            let departure = activity.estimate_departure(route_ctx.route(), a, arrival).unwrap_value();

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

    let tour_len = route_ctx.route().tour.total();
    let (route, state) = route_ctx.as_mut();
    let schedule_states = state.prepare_schedule_states(tour_len);

    route.tour.all_activities().rev().fold(init, |acc, act| {
        if act.job.is_none() {
            schedule_states.push(ActivityScheduleState::default());
            return acc;
        }

        let (end_time, prev_loc, waiting) = acc;
        let latest_arrival_time = if end_time == Float::MAX {
            act.place.time.end
        } else {
            let latest_departure =
                end_time - transport.duration(route, act.place.location, prev_loc, TravelTime::Arrival(end_time));
            activity.estimate_arrival(route, act, latest_departure).unwrap_value()
        };
        let future_waiting = waiting + (act.place.time.start - act.schedule.arrival).max(0.);

        schedule_states
            .push(ActivityScheduleState { latest_arrival: latest_arrival_time, waiting_time: future_waiting });

        (latest_arrival_time, act.place.location, future_waiting)
    });

    schedule_states.reverse();

    // NOTE: pop out state for arrival
    if route.tour.end().is_some_and(|end| end.job.is_none()) {
        schedule_states.pop();
    }
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

    state.update_tour_state::<TotalDistanceTourStateKey, _>(total_dist);
    state.update_tour_state::<TotalDurationTourStateKey, _>(total_dur);
}
