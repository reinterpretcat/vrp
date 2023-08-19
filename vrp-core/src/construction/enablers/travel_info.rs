use crate::construction::heuristics::{ActivityContext, RouteContext};
use crate::models::common::{Distance, Duration, Timestamp};
use crate::models::problem::{TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};

/// Calculates a travel info from prev to next directly.
pub fn calculate_travel(
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    transport: &(dyn TransportCost + Send + Sync),
) -> (Distance, Duration) {
    let route = route_ctx.route();
    let prev = activity_ctx.prev;
    let tar = activity_ctx.target;
    let next = activity_ctx.next;

    let prev_dep = prev.schedule.departure;

    let (prev_to_tar_dis, prev_to_tar_dur) = calculate_travel_leg(route, prev, tar, prev_dep, transport);
    if next.is_none() {
        return (prev_to_tar_dis, prev_to_tar_dur);
    }

    let next = next.unwrap();
    let tar_dep = prev_dep + prev_to_tar_dur;

    let (tar_to_next_dis, tar_to_next_dur) = calculate_travel_leg(route, tar, next, tar_dep, transport);

    (prev_to_tar_dis + tar_to_next_dis, prev_to_tar_dur + tar_to_next_dur)
}

/// Calculates delta in distance and duration for target activity in given activity context.
pub fn calculate_travel_delta(
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    transport: &(dyn TransportCost + Send + Sync),
) -> (Distance, Duration) {
    // NOTE accept some code duplication between methods in that module as they are called often,
    //      generalization might require some redundancy in calculations

    let route = route_ctx.route();
    let prev = activity_ctx.prev;
    let tar = activity_ctx.target;
    let next = activity_ctx.next;

    let prev_dep = prev.schedule.departure;

    let (prev_to_tar_dis, prev_to_tar_dur) = calculate_travel_leg(route, prev, tar, prev_dep, transport);
    if next.is_none() {
        return (prev_to_tar_dis, prev_to_tar_dur);
    }

    let next = next.unwrap();
    let tar_dep = prev_dep + prev_to_tar_dur;

    let (prev_to_next_dis, prev_to_next_dur) = calculate_travel_leg(route, prev, next, prev_dep, transport);
    let (tar_to_next_dis, tar_to_next_dur) = calculate_travel_leg(route, tar, next, tar_dep, transport);

    (prev_to_tar_dis + tar_to_next_dis - prev_to_next_dis, prev_to_tar_dur + tar_to_next_dur - prev_to_next_dur)
}

/// Calculates a travel leg info.
fn calculate_travel_leg(
    route: &Route,
    first: &Activity,
    second: &Activity,
    departure: Timestamp,
    transport: &(dyn TransportCost + Send + Sync),
) -> (Distance, Duration) {
    let first_to_second_dis =
        transport.distance(route, first.place.location, second.place.location, TravelTime::Departure(departure));
    let first_to_second_dur =
        transport.duration(route, first.place.location, second.place.location, TravelTime::Departure(departure));

    let second_arr = departure + first_to_second_dur;
    let second_wait = (second.place.time.start - second_arr).max(0.);
    let second_dep = second_arr + second_wait + second.place.duration;

    (first_to_second_dis, second_dep - departure)
}
