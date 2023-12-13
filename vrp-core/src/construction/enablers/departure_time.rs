#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/departure_time_test.rs"]
mod departure_time_test;

use crate::construction::enablers::{update_route_departure, ScheduleStateKeys};
use crate::construction::features::*;
use crate::construction::heuristics::RouteContext;
use crate::models::common::Timestamp;
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use crate::models::StateKey;
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;

/// Tries to move forward route's departure time.
pub fn advance_departure_time(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    consider_whole_tour: bool,
    state_keys: &ScheduleStateKeys,
) {
    if let Some(new_departure_time) = try_advance_departure_time(route_ctx, transport, consider_whole_tour) {
        update_route_departure(route_ctx, activity, transport, new_departure_time, state_keys);
    }
}

/// Tries to move backward route's departure time.
pub fn recede_departure_time(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost + Send + Sync),
    transport: &(dyn TransportCost + Send + Sync),
    state_keys: &ScheduleStateKeys,
) {
    if let Some(new_departure_time) = try_recede_departure_time(route_ctx, state_keys, LIMIT_DURATION_KEY) {
        update_route_departure(route_ctx, activity, transport, new_departure_time, state_keys);
    }
}

fn try_advance_departure_time(
    route_ctx: &RouteContext,
    transport: &(dyn TransportCost + Send + Sync),
    optimize_whole_tour: bool,
) -> Option<Timestamp> {
    let route = route_ctx.route();

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

        #[allow(clippy::manual_clamp)]
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
    state_keys: &ScheduleStateKeys,
    limit_duration_key: StateKey,
) -> Option<Timestamp> {
    let first = route_ctx.route().tour.get(1)?;
    let start = route_ctx.route().tour.start()?;

    let max_change =
        *route_ctx.state().get_activity_state::<f64>(state_keys.latest_arrival, 1)? - first.schedule.arrival;

    let earliest_allowed_departure =
        route_ctx.route().actor.detail.start.as_ref().and_then(|s| s.time.earliest).unwrap_or(start.place.time.start);

    let max_change = (start.schedule.departure - earliest_allowed_departure).min(max_change);

    let max_change = route_ctx
        .state()
        .get_route_state::<f64>(state_keys.total_duration)
        .zip(route_ctx.state().get_route_state::<f64>(limit_duration_key))
        .map(|(&total, &limit)| (limit - total).min(max_change))
        .unwrap_or(max_change);

    match compare_floats(max_change, 0.) {
        Ordering::Greater => Some(start.schedule.departure - max_change),
        _ => None,
    }
}
