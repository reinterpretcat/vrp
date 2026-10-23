#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/departure_time_test.rs"]
mod departure_time_test;

use crate::construction::enablers::*;
use crate::construction::heuristics::RouteContext;
use crate::models::common::{Duration, TimeSpan, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost, TravelTime};
use crate::models::solution::Route;
use rosomaxa::prelude::Float;

/// Slack on the duration limit comparison, so that a move clamped to land exactly on the limit is
/// not thrown away by the last bit of the recomputation.
const LIMIT_EPSILON: Duration = 1e-6;

/// Tries to move forward route's departure time.
pub fn advance_departure_time(
    route_ctx: &mut RouteContext,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    consider_whole_tour: bool,
) {
    let Some(upper) = try_advance_departure_time(route_ctx, activity, transport, consider_whole_tour) else {
        return;
    };

    let current = route_ctx.route().tour.start().unwrap().schedule.departure;
    let duration_before = route_ctx.state().get_total_duration().copied();

    // Fast path: try the upper bound directly
    update_route_departure(route_ctx, activity, transport, upper);
    if is_departure_acceptable(route_ctx, activity, transport, duration_before) {
        return;
    }

    // Slow path: compute critical departure points and try from highest to lowest
    let candidates = compute_critical_departures(route_ctx.route(), current, upper);
    for &candidate in candidates.iter().rev() {
        if candidate <= current || candidate >= upper {
            continue;
        }
        update_route_departure(route_ctx, activity, transport, candidate);
        if is_departure_acceptable(route_ctx, activity, transport, duration_before) {
            return;
        }
    }

    // Before giving up, leave just in time for the first job. The whole-tour target
    // pushes the departure as far as the waiting further down the tour allows, which a fixed
    // reserved break can make infeasible, and the critical departures above only cover breaks given
    // as offsets. Without this the route keeps the departure it had — the shift's `earliest` after
    // `apply_insertion_success` put it back there for a job inserted in front — and on a
    // `FirstJobTo*` span every second idled before the first job is then paid for: a route leaving
    // at midnight for an 08:00 appointment carries eight hours the duration constraint never saw,
    // because it accepted the insertion against this very departure (`receded_departure` in
    // `tour_limits.rs`).
    for candidate in first_job_departures(route_ctx.route(), activity, transport, current) {
        if candidate <= current || candidate >= upper {
            continue;
        }
        update_route_departure(route_ctx, activity, transport, candidate);
        if is_departure_acceptable(route_ctx, activity, transport, duration_before) {
            return;
        }
    }

    // Neither reaches the first job without running into a break either: the break then falls into
    // the drive or the service, and only a departure early enough to take it in the idle in front of
    // the job holds. That is a boundary on the clock no single formula gives, so it is searched for
    // between the departure the route has, which holds, and the one it wanted.
    if let Some(candidate) = latest_feasible_departure(route_ctx, activity, transport, current, upper) {
        update_route_departure(route_ctx, activity, transport, candidate);
        if is_departure_acceptable(route_ctx, activity, transport, duration_before) {
            return;
        }
    }

    // Fallback: restore current departure
    update_route_departure(route_ctx, activity, transport, current);
}

/// Whether a departure that has just been written can be kept: the schedule it produces has to hold
/// together, and the tour it produces must not be worse against the shift's duration limit than the
/// one the route had before the move.
///
/// Moving a departure forward normally shortens a tour, which is why only the receding side used to
/// weigh the limit (`try_recede_departure_time`). With reserved time it can lengthen one: a break
/// that fell into idling is charged only for the part the idling covered, and once the idling is
/// gone it is charged in full — a charge that can outgrow the stretch the departure moved by. All
/// three callers of `advance_departure_time` run after the duration constraint has had its last word
/// on the move (`apply_insertion`, the `AdvanceDeparture` post-processing and the reschedule local
/// operator), so a tour pushed past its cap here would never be seen again.
///
/// The test is monotone, not absolute. A route can already sit above its cap — a supplied initial
/// solution, or the one `TravelLimitState::notify_failure` forces open — and a move that shortens
/// such a tour toward the cap has to be kept rather than refused for not reaching it.
///
/// `update_route_departure` has already rewritten the schedule and the statistics behind it, so the
/// tour's duration is the one this departure really produces.
fn is_departure_acceptable(
    route_ctx: &RouteContext,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    duration_before: Option<Duration>,
) -> bool {
    if !is_departure_feasible(route_ctx.route(), activity, transport) {
        return false;
    }

    let Some((&total_duration, &limit_duration)) =
        route_ctx.state().get_total_duration().zip(route_ctx.state().get_limit_duration())
    else {
        return true;
    };

    // EPSILON: `try_recede_departure_time` clamps its move by `limit - total`, so the recomputed
    // total lands exactly on the limit and a last-bit overshoot would throw the whole move away.
    total_duration <= limit_duration + LIMIT_EPSILON
        || duration_before.is_some_and(|duration_before| total_duration <= duration_before)
}

/// Whether the schedule a departure produces holds: no reserved time pushes work past the end of
/// its window (`is_schedule_feasible`), and no activity is reached after its window has closed.
///
/// The second half is not implied by the first. A break that falls on a drive lengthens the drive
/// (`DynamicTransportCost`), and the vehicle then arrives late without any activity cost refusing
/// it: every departure computed here assumes the drive it measured at another time of day.
fn is_departure_feasible(route: &Route, activity: &dyn ActivityCost, transport: &dyn TransportCost) -> bool {
    const EPSILON: Duration = 1e-6;

    is_schedule_feasible(route, activity, transport)
        && route.tour.all_activities().skip(1).all(|a| a.schedule.arrival <= a.place.time.end + EPSILON)
}

/// Tries to move backward route's departure time.
pub fn recede_departure_time(route_ctx: &mut RouteContext, activity: &dyn ActivityCost, transport: &dyn TransportCost) {
    let Some(new_departure_time) = try_recede_departure_time(route_ctx) else {
        return;
    };

    let current = route_ctx.route().tour.start().unwrap().schedule.departure;
    let duration_before = route_ctx.state().get_total_duration().copied();

    update_route_departure(route_ctx, activity, transport, new_departure_time);
    // `try_recede_departure_time` clamps the move arithmetically, which assumes the tour grows by
    // exactly the stretch the departure moved back. Reserved time can break that assumption in
    // either direction, so the result is weighed once more here.
    if is_departure_acceptable(route_ctx, activity, transport, duration_before) {
        return;
    }

    // Infeasible or past the limit: restore current departure
    update_route_departure(route_ctx, activity, transport, current);
}

/// Waiting is measured against the service start the schedule passes actually enforce, which
/// `ActivityCost` owns — not against the activity's own window. A shift that declares appointment
/// bounds holds service back past that window, and a departure optimiser blind to it leaves the
/// vehicle idling at its first appointment instead of leaving the depot later; the idle then sits
/// inside the route's duration and eats the shift's own duration limit. For a vehicle that declares
/// no such bounds the default service start is the window opening, so nothing moves.
fn try_advance_departure_time(
    route_ctx: &RouteContext,
    activity_cost: &dyn ActivityCost,
    transport: &dyn TransportCost,
    optimize_whole_tour: bool,
) -> Option<Timestamp> {
    let route = route_ctx.route();

    let first = route.tour.get(1)?;
    let start = route.tour.start()?;

    let latest_allowed_departure = route.actor.detail.start.as_ref().and_then(|s| s.time.latest).unwrap_or(Float::MAX);
    let last_departure_time = start.schedule.departure;

    let new_departure_time = if optimize_whole_tour {
        let (total_waiting_time, max_shift) =
            route.tour.all_activities().rev().fold((0., Float::MAX), |(total_waiting_time, max_shift), activity| {
                let service_start = activity_cost.estimate_service_start(route, activity, activity.schedule.arrival);
                let waiting_time = (service_start - activity.schedule.arrival).max(0.);
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
        let service_start = activity_cost.estimate_service_start(route, first, last_departure_time + start_to_first);

        #[allow(clippy::manual_clamp)]
        last_departure_time.max(service_start - start_to_first).min(latest_allowed_departure)
    };

    if new_departure_time > last_departure_time { Some(new_departure_time) } else { None }
}

/// The departures that reach the first job just in time from `departure`, latest first: once for
/// the moment its service can really start (`estimate_service_start`, which knows the shift's
/// appointment bounds and visit windows) and once for the opening of its own window, the one the
/// duration constraint replays from when it accepts a job in front of the first one
/// (`receded_departure`). The earlier one idles a little in front of the job, but it is the one the
/// constraint has priced.
fn first_job_departures(
    route: &Route,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    departure: Timestamp,
) -> Vec<Timestamp> {
    let (Some(start), Some(first)) = (route.tour.start(), route.tour.get(1)) else {
        return vec![];
    };

    let travel =
        transport.duration(route, start.place.location, first.place.location, TravelTime::Departure(departure));
    let service_start = activity.estimate_service_start(route, first, departure + travel);

    let mut candidates = vec![service_start - travel, first.place.time.start - travel];
    candidates.sort_by(|a, b| b.total_cmp(a));
    candidates.dedup();
    candidates
}

/// The latest whole-second departure between `feasible` and `infeasible` whose schedule holds,
/// found by bisection, or `None` when the search does not get past `feasible`.
///
/// Feasibility is not monotone in the departure in general — a later departure can carry a break
/// past the service it used to collide with — so this finds *a* boundary rather than the latest
/// feasible departure there is. Whatever it returns holds, and the caller still weighs it against
/// the duration limit.
fn latest_feasible_departure(
    route_ctx: &mut RouteContext,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    feasible: Timestamp,
    infeasible: Timestamp,
) -> Option<Timestamp> {
    let (mut lo, mut hi) = (feasible, infeasible);

    while hi - lo > 1. {
        let mid = ((lo + hi) / 2.).floor().max(lo + 1.).min(hi - 1.);

        update_route_departure(route_ctx, activity, transport, mid);
        if is_departure_feasible(route_ctx.route(), activity, transport) {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    (lo > feasible).then_some(lo)
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

    if max_change > 0. { Some(start.schedule.departure - max_change) } else { None }
}

/// Computes critical departure time candidates where feasibility transitions may occur.
/// These are departure values where break boundaries align exactly with job time window boundaries.
fn compute_critical_departures(route: &Route, current: Timestamp, upper: Timestamp) -> Vec<Timestamp> {
    const EPSILON: f64 = 1e-6;

    // Collect break offset info from route activities
    let break_offsets: Vec<(f64, f64, f64)> = route
        .tour
        .all_activities()
        .filter_map(|a| {
            let job = a.job.as_ref()?;
            let place = job.places.get(a.place.idx)?;
            match place.times.iter().find(|t| matches!(t, TimeSpan::Offset(_)))? {
                TimeSpan::Offset(offset) => Some((offset.start, offset.end, place.duration)),
                _ => None,
            }
        })
        .collect();

    if break_offsets.is_empty() {
        return vec![];
    }

    // Collect job TW boundaries from activities with fixed time windows
    let job_tw_boundaries: Vec<f64> = route
        .tour
        .all_activities()
        .filter(|a| a.job.is_some())
        .filter(|a| {
            a.job
                .as_ref()
                .and_then(|j| j.places.get(a.place.idx))
                .map(|p| p.times.iter().any(|t| matches!(t, TimeSpan::Window(_))))
                .unwrap_or(false)
        })
        .flat_map(|a| [a.place.time.start, a.place.time.end])
        .collect();

    let mut candidates = Vec::new();
    for &(offset_start, offset_end, break_dur) in &break_offsets {
        for &tw_boundary in &job_tw_boundaries {
            // D + offset_end + break_dur = tw_boundary
            let d = tw_boundary - offset_end - break_dur;
            push_candidate(&mut candidates, d, current, upper, EPSILON);

            // D + offset_end = tw_boundary
            let d = tw_boundary - offset_end;
            push_candidate(&mut candidates, d, current, upper, EPSILON);

            // D + offset_start = tw_boundary
            let d = tw_boundary - offset_start;
            push_candidate(&mut candidates, d, current, upper, EPSILON);
        }
    }

    candidates.sort_by(|a, b| a.total_cmp(b));
    candidates.dedup();
    candidates
}

fn push_candidate(candidates: &mut Vec<Timestamp>, d: Timestamp, current: Timestamp, upper: Timestamp, epsilon: f64) {
    for &offset in &[-epsilon, 0., epsilon] {
        let val = d + offset;
        if val > current && val < upper {
            candidates.push(val);
        }
    }
}
