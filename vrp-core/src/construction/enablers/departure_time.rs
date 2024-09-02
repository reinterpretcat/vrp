#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/departure_time_test.rs"]
mod departure_time_test;

use crate::construction::enablers::*;
use crate::construction::heuristics::RouteContext;
use crate::models::common::{Duration, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};

/// Tries to move forward route's departure time.
pub fn advance_departure_time(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost),
    transport: &(dyn TransportCost),
    consider_whole_tour: bool,
) {
    if let Some(new_departure_time) = try_advance_departure_time(route_ctx, transport, consider_whole_tour) {
        update_route_departure(route_ctx, activity, transport, new_departure_time);
    }
}

/// Tries to move backward route's departure time.
pub fn recede_departure_time(
    route_ctx: &mut RouteContext,
    activity: &(dyn ActivityCost),
    transport: &(dyn TransportCost),
) {
    if let Some(new_departure_time) = try_recede_departure_time(route_ctx) {
        update_route_departure(route_ctx, activity, transport, new_departure_time);
    }
}

fn try_advance_departure_time(
    route_ctx: &RouteContext,
    transport: &(dyn TransportCost),
    optimize_whole_tour: bool,
) -> Option<Timestamp> {
    let route = route_ctx.route();

    let first = route.tour.get(1)?;
    let start = route.tour.start()?;

    let latest_allowed_departure =
        route.actor.detail.start.as_ref().and_then(|s| s.time.latest).unwrap_or(Timestamp::MAX);
    let last_departure_time = start.schedule.departure;

    let new_departure_time = if optimize_whole_tour {
        let (total_waiting_time, max_shift) = route.tour.all_activities().rev().fold(
            (Duration::default(), Duration::MAX),
            |(total_waiting_time, max_shift), activity| {
                let waiting_time = (activity.place.time.start - activity.schedule.arrival).max(Duration::default());
                let remaining_time =
                    (activity.place.time.end - activity.schedule.arrival - waiting_time).max(Duration::default());

                (total_waiting_time + waiting_time, waiting_time + remaining_time.min(max_shift))
            },
        );
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

fn try_recede_departure_time(route_ctx: &RouteContext) -> Option<Timestamp> {
    let first = route_ctx.route().tour.get(1)?;
    let start = route_ctx.route().tour.start()?;

    let max_change = *route_ctx.state().get_latest_arrival_at(1)? - first.schedule.arrival;

    let earliest_allowed_departure =
        route_ctx.route().actor.detail.start.as_ref().and_then(|s| s.time.earliest).unwrap_or(start.place.time.start);

    let max_change = (start.schedule.departure - earliest_allowed_departure).min(max_change);

    let max_change = route_ctx
        .state()
        .get_total_duration()
        .zip(route_ctx.state().get_limit_duration())
        .map(|(&total, &limit)| (limit - total).min(max_change))
        .unwrap_or(max_change);

    if max_change > Duration::default() {
        Some(start.schedule.departure - max_change)
    } else {
        None
    }
}
