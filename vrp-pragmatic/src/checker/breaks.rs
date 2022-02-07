#[cfg(test)]
#[path = "../../tests/unit/checker/breaks_test.rs"]
mod breaks_test;

use super::*;
use crate::utils::combine_error_results;

/// Checks that breaks are properly assigned.
pub fn check_breaks(context: &CheckerContext) -> Result<(), Vec<String>> {
    combine_error_results(&[check_break_assignment(context)])
}

fn check_break_assignment(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let vehicle_shift = context.get_vehicle_shift(tour)?;
        let actual_break_count = tour
            .stops
            .iter()
            .flat_map(|stop| stop.activities.iter())
            .filter(|activity| activity.activity_type == "break")
            .count();
        let matched_break_count = tour.stops.iter().try_fold(0, |acc, stop| {
            stop.activities
                .windows(stop.activities.len().min(2))
                .flat_map(|leg| as_leg_info_with_break(context, tour, stop, leg))
                .try_fold(acc, |acc, (from_loc, from, to, vehicle_break)| {
                    // check time
                    let visit_time = get_time_window(stop, to);
                    let break_time_window = get_break_time_window(tour, &vehicle_break)?;
                    if !visit_time.intersects(&break_time_window) {
                        return Err(format!(
                            "break visit time '{:?}' is invalid: expected is in '{:?}'",
                            visit_time, break_time_window
                        ));
                    }

                    // check location
                    let actual_loc = context.get_activity_location(stop, to);
                    let backward_loc = from
                        .and_then(|activity| activity.commute.as_ref())
                        .and_then(|commute| commute.backward.as_ref())
                        .map(|info| &info.location);

                    let optional_break_places = match vehicle_break {
                        VehicleBreak::Optional { places, .. } => places,
                        VehicleBreak::Required { .. } => unimplemented!(),
                    };

                    // TODO check tag and duration
                    let has_match = optional_break_places.iter().any(|place| match &place.location {
                        Some(location) => actual_loc == *location,
                        None => *from_loc == actual_loc || backward_loc.map_or(false, |loc| *loc == actual_loc),
                    });

                    if !has_match {
                        return Err(format!(
                            "break location '{:?}' is invalid: cannot match to any break place'",
                            actual_loc
                        ));
                    }
                    Ok(acc + 1)
                })
        })?;

        if actual_break_count != matched_break_count {
            return Err(format!(
                "cannot match all breaks, matched: '{}', actual '{}' for vehicle '{}', shift index '{}'",
                matched_break_count, actual_break_count, tour.vehicle_id, tour.shift_index
            ));
        }

        let departure = tour
            .stops
            .first()
            .map(|stop| parse_time(&stop.time.departure))
            .ok_or_else(|| format!("cannot get departure for tour '{}'", tour.vehicle_id))?;

        let arrival = tour
            .stops
            .last()
            .map(|stop| parse_time(&stop.time.arrival))
            .ok_or_else(|| format!("cannot get arrival for tour '{}'", tour.vehicle_id))?;

        let tour_tw = TimeWindow::new(departure, arrival);

        let expected_break_count =
            vehicle_shift.breaks.iter().flat_map(|breaks| breaks.iter()).fold(0, |acc, vehicle_break| {
                let break_tw = get_break_time_window(tour, vehicle_break).expect("cannot get break time windows");

                let should_assign = match vehicle_break {
                    VehicleBreak::Optional { policy, .. } => {
                        let policy = policy.as_ref().cloned().unwrap_or(VehicleBreakPolicy::SkipIfNoIntersection);

                        match policy {
                            VehicleBreakPolicy::SkipIfNoIntersection => break_tw.start < arrival,
                            VehicleBreakPolicy::SkipIfArrivalBeforeEnd => arrival > break_tw.end,
                        }
                    }
                    VehicleBreak::Required { .. } => break_tw.intersects(&tour_tw),
                };

                if should_assign {
                    acc + 1
                } else {
                    acc
                }
            });

        let total_break_count = actual_break_count + get_break_violation_count(&context.solution, tour);

        if expected_break_count != total_break_count {
            Err(format!(
                "amount of breaks does not match, expected: '{}', got '{}' for vehicle '{}', shift index '{}'",
                expected_break_count, total_break_count, tour.vehicle_id, tour.shift_index
            ))
        } else {
            Ok(())
        }
    })
}

fn as_leg_info_with_break<'a>(
    context: &CheckerContext,
    tour: &Tour,
    stop: &'a Stop,
    leg: &'a [Activity],
) -> Option<(&'a Location, Option<&'a Activity>, &'a Activity, VehicleBreak)> {
    let leg = match leg {
        [from, to] => Some((Some(from), to)),
        [to] => Some((None, to)),
        _ => None,
    };

    if let Some((from, to)) = leg {
        if let Ok(ActivityType::Break(vehicle_break)) = context.get_activity_type(tour, stop, to) {
            let from_loc =
                leg.and_then(|(from, _)| from).and_then(|action| action.location.as_ref()).unwrap_or(&stop.location);
            return Some((from_loc, from, to, vehicle_break));
        }
    }
    None
}

fn get_break_time_window(tour: &Tour, vehicle_break: &VehicleBreak) -> Result<TimeWindow, String> {
    let departure = tour
        .stops
        .first()
        .map(|stop| parse_time(&stop.time.departure))
        .ok_or_else(|| format!("cannot get departure time for tour: '{}'", tour.vehicle_id))?;

    match vehicle_break {
        VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeWindow(tw), .. } => Ok(parse_time_window(tw)),
        VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeOffset(offset), .. } => {
            if offset.len() != 2 {
                return Err(format!("invalid offset break for tour: '{}'", tour.vehicle_id));
            }

            Ok(TimeWindow::new(departure + *offset.first().unwrap(), departure + *offset.last().unwrap()))
        }
        VehicleBreak::Required { time, duration } => {
            let time = match time {
                VehicleRequiredBreakTime::OffsetTime(offset) => *offset,
                VehicleRequiredBreakTime::ExactTime(time) => parse_time(time),
            };

            let start = departure + time;
            Ok(TimeWindow::new(start, start + duration))
        }
    }
}

fn get_break_violation_count(solution: &Solution, tour: &Tour) -> usize {
    solution.violations.as_ref().map_or(0, |violations| {
        violations
            .iter()
            .filter(|v| match v {
                Violation::Break { vehicle_id, shift_index, .. }
                    if *vehicle_id == tour.vehicle_id && *shift_index == tour.shift_index =>
                {
                    true
                }
                _ => false,
            })
            .count()
    })
}
