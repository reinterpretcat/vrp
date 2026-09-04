#[cfg(test)]
#[path = "../../tests/unit/checker/breaks_test.rs"]
mod breaks_test;

use super::*;
use crate::format::problem::RouteCostSpan as FmtRouteCostSpan;
use crate::utils::combine_error_results;
use std::iter::once;
use vrp_core::models::common::Timestamp;
use vrp_core::prelude::GenericResult;
use vrp_core::utils::GenericError;

/// Checks that breaks are properly assigned.
pub fn check_breaks(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_break_assignment(context)])
}

fn check_break_assignment(context: &CheckerContext) -> GenericResult<()> {
    context.solution.tours.iter().try_for_each(|tour| {
        let vehicle_shift = context.get_vehicle_shift(tour)?;
        let cost_span = context.get_vehicle(&tour.vehicle_id).ok().and_then(|v| v.costs.span.as_ref());

        let actual_break_count = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities().iter())
            .filter(|activity| activity.activity_type == "break")
            .count();
        let matched_break_count = tour.stops.iter().try_fold(0, |acc, stop| {
            stop.activities()
                .windows(stop.activities().len().min(2))
                .flat_map(|leg| as_leg_info_with_break(context, tour, stop, leg))
                .try_fold::<_, _, GenericResult<_>>(
                    acc,
                    |acc, (from_loc, (from, to), (break_activity, vehicle_break))| {
                        // check time
                        let visit_time = get_time_window(stop, break_activity);
                        let break_time_window = get_break_time_window(tour, &vehicle_break, cost_span)?;
                        if !visit_time.intersects(&break_time_window) {
                            return Err(format!(
                                "break visit time '{visit_time:?}' is invalid: expected is in '{break_time_window:?}'",
                            )
                            .into());
                        }

                        // check location
                        let actual_loc = context.get_activity_location(stop, to);
                        let backward_loc = from
                            .and_then(|activity| activity.commute.as_ref())
                            .and_then(|commute| commute.backward.as_ref())
                            .map(|info| &info.location)
                            .cloned();

                        let has_match = match vehicle_break {
                            // TODO check tag and duration
                            VehicleBreak::Optional { places, .. } => places.iter().any(|place| match &place.location {
                                Some(location) => actual_loc.as_ref() == Some(location),
                                None => from_loc == actual_loc || backward_loc == actual_loc,
                            }),
                            VehicleBreak::Required { .. } => actual_loc.is_none() || from_loc == actual_loc,
                        };

                        if !has_match {
                            return Err(format!(
                                "break location '{actual_loc:?}' is invalid: cannot match to any break place'"
                            )
                            .into());
                        }
                        Ok(acc + 1)
                    },
                )
        })?;

        if actual_break_count != matched_break_count {
            return Err(format!(
                "cannot match all breaks, matched: '{}', actual '{}' for vehicle '{}', shift index '{}'",
                matched_break_count, actual_break_count, tour.vehicle_id, tour.shift_index
            )
            .into());
        }

        let departure = tour
            .stops
            .first()
            .map(|stop| parse_time(&stop.schedule().departure))
            .ok_or_else(|| GenericError::from(format!("cannot get departure for tour '{}'", tour.vehicle_id)))?;

        let arrival = tour
            .stops
            .last()
            .map(|stop| parse_time(&stop.schedule().arrival))
            .ok_or_else(|| GenericError::from(format!("cannot get arrival for tour '{}'", tour.vehicle_id)))?;

        let tour_tw = TimeWindow::new(departure, arrival);

        let expected_break_count =
            vehicle_shift.breaks.iter().flat_map(|breaks| breaks.iter()).fold(0, |acc, vehicle_break| {
                let break_tw =
                    get_break_time_window(tour, vehicle_break, cost_span).expect("cannot get break time windows");

                let should_assign = match vehicle_break {
                    VehicleBreak::Optional { policy, .. } => {
                        let policy =
                            policy.as_ref().cloned().unwrap_or(VehicleOptionalBreakPolicy::SkipIfNoIntersection);

                        match policy {
                            VehicleOptionalBreakPolicy::SkipIfNoIntersection => break_tw.start < arrival,
                            VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd => arrival > break_tw.end,
                        }
                    }
                    VehicleBreak::Required { .. } => {
                        // NOTE: skip break if its end time is after tour end
                        break_tw.intersects(&tour_tw) && break_tw.end < tour_tw.end
                    }
                };

                if should_assign { acc + 1 } else { acc }
            });

        let total_break_count = actual_break_count + get_break_violation_count(&context.solution, tour);

        if expected_break_count != total_break_count {
            Err(format!(
                "amount of breaks does not match, expected: '{}', got '{}' for vehicle '{}', shift index '{}'",
                expected_break_count, total_break_count, tour.vehicle_id, tour.shift_index
            )
            .into())
        } else {
            Ok(())
        }
    })
}

/// Represents information about break and neighbour activity.
type LegBreakInfo<'a> = (Option<Location>, (Option<&'a Activity>, &'a Activity), (&'a Activity, VehicleBreak));

fn as_leg_info_with_break<'a>(
    context: &CheckerContext,
    tour: &Tour,
    stop: &'a Stop,
    leg: &'a [Activity],
) -> Option<LegBreakInfo<'a>> {
    let leg = match leg {
        [from, to] => Some((Some(from), to)),
        [to] => Some((None, to)),
        _ => None,
    };

    if let Some((from, to)) = leg
        && let Some((break_activity, vehicle_break)) = once(to)
            .chain(from.iter().cloned())
            .flat_map(|activity| context.get_activity_type(tour, stop, activity).map(|at| (activity, at)))
            .filter_map(|(activity, activity_type)| match activity_type {
                ActivityType::Break(vehicle_break) => Some((activity, vehicle_break)),
                _ => None,
            })
            .next()
    {
        let from_loc = leg.and_then(|(from, _)| from).and_then(|action| action.location.as_ref()).or(match stop {
            Stop::Point(point) => Some(&point.location),
            Stop::Transit(_) => None,
        });
        return Some((from_loc.cloned(), (from, to), (break_activity, vehicle_break)));
    }
    None
}

/// Gets break time window, using the RouteCostSpan to determine the anchor for offset breaks.
pub(crate) fn get_break_time_window(
    tour: &Tour,
    vehicle_break: &VehicleBreak,
    cost_span: Option<&FmtRouteCostSpan>,
) -> GenericResult<TimeWindow> {
    let departure = tour
        .stops
        .first()
        .map(|stop| parse_time(&stop.schedule().departure))
        .ok_or_else(|| format!("cannot get departure time for tour: '{}'", tour.vehicle_id))?;

    // Compute the offset anchor based on RouteCostSpan
    let offset_anchor = match cost_span {
        Some(FmtRouteCostSpan::FirstJobToDepot | FmtRouteCostSpan::FirstJobToLastJob) => {
            get_first_job_arrival(tour).unwrap_or(departure)
        }
        _ => departure,
    };

    match vehicle_break {
        VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeWindow(tw), .. } => Ok(parse_time_window(tw)),
        VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeOffset(offset), .. } => {
            if offset.len() != 2 {
                return Err(format!("invalid offset break for tour: '{}'", tour.vehicle_id).into());
            }

            Ok(TimeWindow::new(departure + *offset.first().unwrap(), departure + *offset.last().unwrap()))
        }
        VehicleBreak::Required { time, duration } => {
            let (start, end) = match time {
                VehicleRequiredBreakTime::OffsetTime { earliest, latest } => {
                    (offset_anchor + *earliest, offset_anchor + *latest)
                }
                VehicleRequiredBreakTime::ExactTime { earliest, latest } => (parse_time(earliest), parse_time(latest)),
            };

            Ok(TimeWindow::new(start, end + duration))
        }
    }
}

/// Gets the arrival time of the first job activity in the tour.
fn get_first_job_arrival(tour: &Tour) -> Option<Timestamp> {
    // The first stop is departure, so first job is the second stop (or first non-departure activity)
    tour.stops
        .iter()
        .flat_map(|stop| stop.activities().iter())
        .find(|a| !matches!(a.activity_type.as_str(), "departure" | "arrival"))
        .and_then(|_| {
            // Find the stop that contains the first job activity and get its arrival
            tour.stops
                .iter()
                .find(|stop| {
                    stop.activities().iter().any(|a| !matches!(a.activity_type.as_str(), "departure" | "arrival"))
                })
                .map(|stop| parse_time(&stop.schedule().arrival))
        })
}

fn get_break_violation_count(solution: &Solution, tour: &Tour) -> usize {
    solution.violations.as_ref().map_or(0, |violations| {
        violations
            .iter()
            .filter(|v| matches!(v, Violation::Break { vehicle_id, shift_index, .. } if *vehicle_id == tour.vehicle_id && *shift_index == tour.shift_index))
            .count()
    })
}
