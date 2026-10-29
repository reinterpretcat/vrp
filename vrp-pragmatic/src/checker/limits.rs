#[cfg(test)]
#[path = "../../tests/unit/checker/limits_test.rs"]
mod limits_test;

use super::*;
use crate::utils::combine_error_results;
use vrp_core::models::common::Distance;
use vrp_core::prelude::GenericResult;

/// NOTE to ensure distance/duration correctness, routing check should be performed first.
pub fn check_limits(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_shift_limits(context), check_shift_time(context), check_recharge_limits(context)])
}

/// Check that shift limits are not violated:
/// * max shift time
/// * max distance
/// * tour size
///
/// NOTE `minTourSize` is deliberately absent. The solver models it as an objective
/// (`min_tour_size_objective`), not a constraint, so an under-sized tour is a worse
/// solution and never an infeasible one. Checking it here reported every solution as
/// broken and drowned the violations that are real.
fn check_shift_limits(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each::<_, GenericResult<_>>(|tour| {
        let vehicle = context.get_vehicle(&tour.vehicle_id)?;

        if let Some(ref limits) = vehicle.limits {
            if let Some(max_distance) = limits.max_distance
                && tour.statistic.distance as Float > max_distance {
                    return Err(format!(
                        "max distance limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        max_distance, tour.statistic.distance, tour.vehicle_id, tour.shift_index
                    ).into());
                }

            if let Some(max_duration) = limits.max_duration
                && let Some(duration) = paid_duration(tour, vehicle)
                && duration > max_duration {
                    return Err(format!(
                        "shift time limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        max_duration, duration, tour.vehicle_id, tour.shift_index
                    ).into());
                }

            if let Some(tour_size_limit) = limits.tour_size {
                // Stops only — the same count the solver's constraint keeps: departure and
                // arrival frame the tour, and a break, reload or recharge is not a visit.
                let tour_activities = tour
                    .stops
                    .iter()
                    .flat_map(|stop| stop.activities())
                    .filter(|activity| is_stop_activity(activity))
                    .count();

                if tour_activities > tour_size_limit {
                    return Err(format!(
                        "tour size limit violation, expected: not more than {}, got: {}, vehicle id '{}', shift index: {}",
                        tour_size_limit, tour_activities, tour.vehicle_id, tour.shift_index
                    ).into())
                }
            }

        }

        Ok(())
    })
}

/// The stretch of the tour the vehicle is paid for, which is the stretch `maxDuration` caps.
///
/// `tour.statistic.duration` is always the round trip — the solution writer says so where it
/// builds it — while the solver enforces the cap over the shift's `costs.span`
/// (`calculate_route_duration` in vrp-core). Read the round trip against a span-trimmed cap and
/// every tour whose commute legs push it past the cap is reported as broken: 865 of 6382 tours on
/// one production month, all but five of them inside their cap on the stretch they are paid for.
/// None of the 865 is something an operator can act on, and the first one ends the check — this
/// function is what stops a real violation from hiding behind them.
///
/// First and last job are the first and last STOP, the same reading the tour size check above
/// takes: a break, a reload or a recharge is not a visit, and the paid span runs between customers.
/// A tour with no stop at all has nothing to measure and is left to `check_shift_time`.
fn paid_duration(tour: &Tour, vehicle: &VehicleType) -> Option<Float> {
    let (start, end) = tour.stops.first().zip(tour.stops.last())?;

    let start_departure = parse_time(&start.schedule().departure);
    let end_departure = parse_time(&end.schedule().departure);

    let is_visit = |stop: &&Stop| stop.activities().iter().any(is_stop_activity);
    let first_arrival = tour.stops.iter().find(is_visit).map(|stop| parse_time(&stop.schedule().arrival));
    let last_departure = tour.stops.iter().rev().find(is_visit).map(|stop| parse_time(&stop.schedule().departure));

    // mirrors `calculate_route_duration`: a tour that visits nobody is charged for nothing, and
    // only the round trip is measurable without a visit to anchor it
    Some(match vehicle.costs.span.clone().unwrap_or_default() {
        RouteCostSpan::DepotToDepot => end_departure - start_departure,
        RouteCostSpan::DepotToLastJob => last_departure.map_or_else(Duration::default, |last| last - start_departure),
        RouteCostSpan::FirstJobToDepot => first_arrival.map_or_else(Duration::default, |first| end_departure - first),
        RouteCostSpan::FirstJobToLastJob => {
            first_arrival.zip(last_departure).map_or_else(Duration::default, |(first, last)| last - first)
        }
    })
}

fn check_shift_time(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each::<_, GenericResult<_>>(|tour| {
        let vehicle = context.get_vehicle(&tour.vehicle_id)?;

        let (start, end) = tour.stops.first().zip(tour.stops.last()).ok_or("empty tour")?;

        let departure = parse_time(&start.schedule().departure);
        let arrival = parse_time(&end.schedule().arrival);

        let has_match = vehicle
            .shifts
            .iter()
            .map(|shift| {
                let start = parse_time(&shift.start.earliest);
                let end = shift.end.as_ref().map(|end| parse_time(&end.latest)).unwrap_or(Float::MAX);

                (start, end)
            })
            .any(|(start, end)| departure >= start && arrival <= end);

        if !has_match {
            Err(format!(
                "tour time is outside shift time, vehicle id '{}', shift index: {}",
                tour.vehicle_id, tour.shift_index
            )
            .into())
        } else {
            Ok(())
        }
    })
}

fn check_recharge_limits(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().filter(|tour| tour.stops.len() > 1).try_for_each::<_, GenericResult<_>>(|tour| {
        let shift = context.get_vehicle_shift(tour)?;

        let Some(recharge) = shift.recharges.as_ref() else { return Ok(()) };

        let stops = tour.stops.iter().filter_map(|stop| stop.as_point()).collect::<Vec<_>>();
        if stops.len() < 2 {
            return Ok(());
        }

        stops
            .windows(2)
            .try_fold(Distance::default(), |acc, stops| {
                let (prev, next) = match stops {
                    [prev, next] => (prev, next),
                    _ => unreachable!(),
                };

                let delta = (next.distance - prev.distance) as Distance;
                let total_distance = acc + delta;

                if total_distance > recharge.max_distance {
                    return Err(format!(
                        "recharge distance violation: expected limit is {}, got {}, vehicle id '{}', shift index: {}",
                        recharge.max_distance, total_distance, tour.vehicle_id, tour.shift_index
                    )
                    .into());
                }

                let has_recharge = next.activities.iter().any(|activity| activity.activity_type == "recharge");

                Ok(if has_recharge { Distance::default() } else { total_distance })
            })
            .map(|_| ())
    })
}
