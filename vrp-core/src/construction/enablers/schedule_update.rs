#[cfg(test)]
#[path = "../../../tests/unit/construction/enablers/schedule_update_test.rs"]
mod schedule_update_test;

use crate::construction::heuristics::{RouteContext, RouteState};
use crate::models::OP_START_MSG;
use crate::models::common::{Distance, Duration, Schedule, TimeSpan, Timestamp};
use crate::models::problem::{ActivityCost, RouteCostSpan, RouteCostSpanDimension, TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};
use rosomaxa::prelude::Float;
use rosomaxa::utils::UnwrapValue;
use std::ops::ControlFlow;

custom_activity_state!(pub(crate) LatestArrival typeof Timestamp);
custom_activity_state!(pub(crate) WaitingTime typeof Timestamp);
custom_tour_state!(pub TotalDistance typeof Distance);
custom_tour_state!(pub TotalDuration typeof Duration);
custom_tour_state!(pub PaidWorkingDuration typeof Duration);
custom_tour_state!(pub(crate) LimitDuration typeof Duration);

/// Updates route schedule data.
pub fn update_route_schedule(route_ctx: &mut RouteContext, activity: &dyn ActivityCost, transport: &dyn TransportCost) {
    let cost_span = route_ctx.route().actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default();
    let needs_fixed_point = matches!(cost_span, RouteCostSpan::FirstJobToDepot | RouteCostSpan::FirstJobToLastJob);

    update_schedules(route_ctx, activity, transport);

    if needs_fixed_point {
        // For FirstJobTo* spans, the offset anchor depends on first_job.arrival which is
        // computed during update_schedules. Re-run if the anchor changed significantly.
        const EPSILON: f64 = 1e-6;
        const MAX_ITERATIONS: usize = 3;

        for _ in 0..MAX_ITERATIONS {
            let anchor = get_offset_anchor(route_ctx.route());
            update_schedules(route_ctx, activity, transport);
            let new_anchor = get_offset_anchor(route_ctx.route());

            if (new_anchor - anchor).abs() <= EPSILON {
                break;
            }
        }
    }

    update_states(route_ctx, activity, transport);
    update_statistics(route_ctx, transport);
}

/// Returns the offset anchor timestamp based on the route's `RouteCostSpan`.
/// For `DepotToDepot`/`DepotToLastJob`, this is the start departure time.
/// For `FirstJobToDepot`/`FirstJobToLastJob`, this is the first job's arrival time (if available).
///
/// "First job" means the first activity that is not itself positioned by this anchor. A required
/// break with an offset span is an activity like any other and can land at index 1, ahead of every
/// customer; anchoring on it would define its own window in terms of itself, and the break would
/// then sit before the first job at whatever time the search happened to give it.
pub fn get_offset_anchor(route: &Route) -> Timestamp {
    let cost_span = route.actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default();
    let start_departure = route.tour.start().map(|a| a.schedule.departure).unwrap_or(0.);

    match cost_span {
        RouteCostSpan::DepotToDepot | RouteCostSpan::DepotToLastJob => start_departure,
        RouteCostSpan::FirstJobToDepot | RouteCostSpan::FirstJobToLastJob => route
            .tour
            .all_activities()
            .skip(1)
            .find(|activity| activity.job.is_some() && !is_offset_anchored(activity))
            .map(|activity| activity.schedule.arrival)
            .unwrap_or(start_departure),
    }
}

/// Whether the activity's chosen window came from an offset span, i.e. it is placed relative to the
/// anchor rather than at a time of its own.
fn is_offset_anchored(activity: &Activity) -> bool {
    activity
        .job
        .as_ref()
        .and_then(|job| job.places.get(activity.place.idx))
        .is_some_and(|place| place.times.iter().any(|span| matches!(span, TimeSpan::Offset(_))))
}

/// Checks whether the route schedule is feasible by simulating the forward pass of `update_schedules`.
/// Returns `true` if no activity produces a `ControlFlow::Break` during departure estimation.
pub fn is_schedule_feasible(route: &Route, activity: &dyn ActivityCost, transport: &dyn TransportCost) -> bool {
    let start = route.tour.start().expect(OP_START_MSG);
    let mut loc = start.place.location;
    let mut dep = start.schedule.departure;

    for activity_idx in 1..route.tour.total() {
        let a = route.tour.get(activity_idx).unwrap();
        let location = a.place.location;
        let arrival = dep + transport.duration(route, loc, location, TravelTime::Departure(dep));

        match activity.estimate_departure(route, a, arrival) {
            ControlFlow::Break(_) => return false,
            ControlFlow::Continue(d) => {
                loc = location;
                dep = d;
            }
        }
    }

    true
}

/// Updates route departure to the new one.
pub fn update_route_departure(
    route_ctx: &mut RouteContext,
    activity: &dyn ActivityCost,
    transport: &dyn TransportCost,
    new_departure_time: Timestamp,
) {
    let old_anchor = get_offset_anchor(route_ctx.route());

    {
        let start = route_ctx.route_mut().tour.get_mut(0).unwrap();
        start.schedule.departure = new_departure_time;
    }

    let new_anchor = get_offset_anchor(route_ctx.route());
    recompute_offset_time_windows(route_ctx, old_anchor, new_anchor);

    update_route_schedule(route_ctx, activity, transport);
}

/// Recomputes activity time windows derived from offset spans after anchor shift.
fn recompute_offset_time_windows(route_ctx: &mut RouteContext, old_anchor: Timestamp, new_anchor: Timestamp) {
    if old_anchor == new_anchor {
        return;
    }

    route_ctx.route_mut().tour.all_activities_mut().for_each(|activity| {
        let Some(job) = activity.job.as_ref() else { return };
        let place_idx = activity.place.idx;

        let Some(place_def) = job.places.get(place_idx) else { return };

        // Only adjust activities whose selected time window came from an offset span.
        let Some(span) = place_def
            .times
            .iter()
            .find(|span| matches!(span, TimeSpan::Offset(_)) && span.to_time_window(old_anchor) == activity.place.time)
        else {
            return;
        };

        activity.place.time = span.to_time_window(new_anchor);
    });
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
            activity.estimate_arrival(route, act, latest_departure).unwrap_value()
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

/// Returns the duration of the route measured over the span its shift is paid on.
///
/// This is the figure `update_statistics` stores as the tour's `TotalDuration` state, and the one
/// anything which prices time has to ask for: a shift paid from its first job to its last one runs
/// shorter than the round trip which carries it, and the depot legs it is not paid for must not be
/// charged to it. Asking here rather than re-deriving the span is what keeps the cost the solver
/// optimises and the cost the solution reports the same number.
pub fn get_route_duration(route: &Route) -> Duration {
    let (Some(start), Some(end)) = (route.tour.start(), route.tour.end()) else {
        return Duration::default();
    };

    let cost_span = route.actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default();

    calculate_route_duration(route, cost_span, route.tour.total(), start, end)
}

/// When the route's paid span starts and ends: the same activities [`get_route_duration`] measures
/// over, by the vehicle's `RouteCostSpan`. `None` for a route without a start and an end, and for a
/// span that runs from or to a job on a route without one.
///
/// The off-hours premium asks this rather than the duration, because what it prices is where the
/// span lies on the clock, not how long it runs.
pub fn get_paid_span(route: &Route) -> Option<(Timestamp, Timestamp)> {
    let (start, end) = (route.tour.start()?, route.tour.end()?);
    let total = route.tour.total();

    match route.actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default() {
        RouteCostSpan::DepotToDepot => Some((start.schedule.departure, end.schedule.departure)),
        RouteCostSpan::DepotToLastJob => {
            let last = route.tour.get(get_last_job_idx(route, total)?)?;
            Some((start.schedule.departure, last.schedule.departure))
        }
        RouteCostSpan::FirstJobToDepot => {
            has_jobs(route, total).then(|| (route.tour.get(1).unwrap().schedule.arrival, end.schedule.departure))
        }
        RouteCostSpan::FirstJobToLastJob => {
            let last = route.tour.get(get_last_job_idx(route, total)?)?;
            Some((route.tour.get(1)?.schedule.arrival, last.schedule.departure))
        }
    }
}

fn update_statistics(route_ctx: &mut RouteContext, transport: &dyn TransportCost) {
    let (route, state) = route_ctx.as_mut();

    // `get_route_duration` is total because the writer must be able to ask it about any route. Here
    // it is not: a route being scheduled without a start and an end is a bug, and one which would
    // otherwise surface as a tour whose duration is quietly zero.
    debug_assert!(route.tour.start().is_some(), "{OP_START_MSG}");
    debug_assert!(route.tour.end().is_some(), "route being scheduled has no end activity");

    let total_activities = route.tour.total();
    let cost_span = route.actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default();

    let total_dur = get_route_duration(route);
    let total_dist = calculate_route_distance(route, transport, total_activities);
    let worked_dur = calculate_paid_working_duration(route, transport, cost_span, total_activities);

    state.set_total_distance(total_dist);
    state.set_total_duration(total_dur);
    state.set_paid_working_duration(worked_dur);
}

/// The paid span with the idle taken out: service plus the drive, over exactly the activities the
/// vehicle's `RouteCostSpan` pays for.
///
/// ⚠️ **Why this exists beside [`calculate_route_duration`].** The span is what the technician is
/// PAID, and it is the right number to bill. It is the wrong number to BALANCE on, because it is
/// bounded by the day rather than driven by the work: a driver already on the clock from eight to
/// six absorbs another job without the span moving, so an objective measuring it has almost no
/// gradient to follow. Measured on a captured 751-job chunk, balancing the span left the fleet's
/// paid hours MORE uneven than not balancing at all (spread 0.14 -> 0.25), while balancing this
/// quantity levelled both it and the span (0.25 -> 0.11).
///
/// Idle is excluded because no assignment creates it on purpose — it is what a time window leaves
/// behind — while service and drive are exactly what an assignment decides.
fn calculate_paid_working_duration(
    route: &Route,
    transport: &dyn TransportCost,
    cost_span: RouteCostSpan,
    total_activities: usize,
) -> Duration {
    let Some((start_idx, end_idx)) = paid_activity_range(route, cost_span, total_activities) else {
        return Duration::default();
    };

    let mut total = Duration::default();
    let mut previous: Option<&Activity> = None;

    for idx in start_idx..end_idx {
        let Some(activity) = route.tour.get(idx) else { continue };

        if activity.job.is_some() {
            total += activity.place.duration;
        }

        if let Some(from) = previous {
            total += transport.duration(
                route,
                from.place.location,
                activity.place.location,
                TravelTime::Departure(from.schedule.departure),
            );
        }

        previous = Some(activity);
    }

    total
}

/// The half-open activity range a `RouteCostSpan` covers — the same four cases
/// [`calculate_route_duration`] measures between, expressed as indices.
fn paid_activity_range(route: &Route, cost_span: RouteCostSpan, total_activities: usize) -> Option<(usize, usize)> {
    if total_activities <= 1 {
        return None;
    }

    let last_job_idx = get_last_job_idx(route, total_activities);

    match cost_span {
        RouteCostSpan::DepotToDepot => Some((0, total_activities)),
        RouteCostSpan::DepotToLastJob => last_job_idx.map(|last| (0, last + 1)),
        RouteCostSpan::FirstJobToDepot => has_jobs(route, total_activities).then_some((1, total_activities)),
        RouteCostSpan::FirstJobToLastJob => last_job_idx.map(|last| (1, last + 1)),
    }
}

/// Returns the index of the last job activity in the route.
/// For closed tours (with end depot): last job is at total - 2
/// For open tours (no end depot): last job is at total - 1
fn get_last_job_idx(route: &Route, total_activities: usize) -> Option<usize> {
    if total_activities <= 1 {
        return None;
    }

    // Check if the last activity is an end depot (job is None) or a job activity
    let end = route.tour.end()?;
    let has_end_depot = end.job.is_none();

    if has_end_depot {
        // Closed tour: [start, job1, ..., jobN, end] - last job at total - 2
        if total_activities > 2 { Some(total_activities - 2) } else { None }
    } else {
        // Open tour: [start, job1, ..., jobN] - last job at total - 1
        Some(total_activities - 1)
    }
}

/// Returns the minimum number of activities required for the route to have jobs.
/// For closed tours: 3 (start, at least one job, end)
/// For open tours: 2 (start, at least one job)
fn has_jobs(route: &Route, total_activities: usize) -> bool {
    let end = route.tour.end();
    let has_end_depot = end.is_some_and(|e| e.job.is_none());

    if has_end_depot { total_activities > 2 } else { total_activities > 1 }
}

fn calculate_route_duration(
    route: &Route,
    cost_span: RouteCostSpan,
    total_activities: usize,
    start: &Activity,
    end: &Activity,
) -> Duration {
    match cost_span {
        RouteCostSpan::DepotToDepot => {
            // For open tours, DepotToDepot is effectively DepotToLastJob
            end.schedule.departure - start.schedule.departure
        }
        RouteCostSpan::DepotToLastJob => {
            if let Some(last_job_idx) = get_last_job_idx(route, total_activities) {
                let last_job = route.tour.get(last_job_idx).unwrap();
                last_job.schedule.departure - start.schedule.departure
            } else {
                Duration::default()
            }
        }
        RouteCostSpan::FirstJobToDepot => {
            // For open tours, there's no depot to return to, so this behaves like FirstJobToLastJob
            if has_jobs(route, total_activities) {
                let first_job = route.tour.get(1).unwrap();
                end.schedule.departure - first_job.schedule.arrival
            } else {
                Duration::default()
            }
        }
        RouteCostSpan::FirstJobToLastJob => {
            if let Some(last_job_idx) = get_last_job_idx(route, total_activities) {
                let first_job = route.tour.get(1).unwrap();
                let last_job = route.tour.get(last_job_idx).unwrap();
                last_job.schedule.departure - first_job.schedule.arrival
            } else {
                Duration::default()
            }
        }
    }
}

fn calculate_route_distance(route: &Route, transport: &dyn TransportCost, total_activities: usize) -> Distance {
    // Deliberately span-blind, unlike `calculate_route_duration`. Miles are vehicle cost — fuel,
    // wear, leasing — and they are incurred on the commute legs whether or not the technician is
    // on the clock for them. The cost model bills the whole route for exactly that reason
    // ("Distance Cost (always full distance, regardless of pay period)"), and a distance trimmed
    // by the pay period would have the solver optimise one quantity while the invoice charges
    // another.
    if total_activities <= 1 {
        return Distance::default();
    }

    let start_activity = route.tour.get(0).unwrap();
    let init = (start_activity.place.location, start_activity.schedule.departure, Distance::default());

    route
        .tour
        .all_activities()
        .skip(1)
        .fold(init, |(loc, dep, total_dist), a| {
            let dist = total_dist + transport.distance(route, loc, a.place.location, TravelTime::Departure(dep));
            (a.place.location, a.schedule.departure, dist)
        })
        .2
}
