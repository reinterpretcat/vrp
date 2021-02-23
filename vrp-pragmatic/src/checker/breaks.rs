#[cfg(test)]
#[path = "../../tests/unit/checker/breaks_test.rs"]
mod breaks_test;

use super::*;

/// Checks that breaks are properly assigned.
pub fn check_breaks(context: &CheckerContext) -> Result<(), String> {
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
                .try_fold(acc, |acc, (from_loc, to, vehicle_break)| {
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
                    let actual_location = get_location(stop, to);
                    match &vehicle_break.locations {
                        Some(locations) => {
                            let is_correct = locations.iter().any(|location| actual_location == *location);
                            if !is_correct {
                                return Err(format!(
                                    "break location '{:?}' is invalid: expected one of '{:?}'",
                                    actual_location, locations
                                ));
                            }
                        }
                        None => {
                            if *from_loc != actual_location {
                                return Err(format!(
                                    "break location '{:?}' is invalid: expected previous activity location '{:?}'",
                                    actual_location, from_loc
                                ));
                            }
                        }
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

        let arrival = tour
            .stops
            .last()
            .map(|stop| parse_time(&stop.time.arrival))
            .ok_or_else(|| format!("cannot get arrival for tour '{}'", tour.vehicle_id))?;

        let expected_break_count =
            vehicle_shift.breaks.iter().flat_map(|breaks| breaks.iter()).fold(0, |acc, vehicle_break| {
                let break_tw = get_break_time_window(tour, vehicle_break).expect("Cannot get break time windows");
                if break_tw.start < arrival {
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
) -> Option<(&'a Location, &'a Activity, VehicleBreak)> {
    let leg = match leg {
        [from, to] => Some((from.location.as_ref().unwrap_or(&stop.location), to)),
        [to] => Some((&stop.location, to)),
        _ => None,
    };

    if let Some((from_loc, to)) = leg {
        if let Ok(ActivityType::Break(vehicle_break)) = context.get_activity_type(tour, stop, to) {
            return Some((from_loc, to, vehicle_break));
        }
    }
    None
}

fn get_break_time_window(tour: &Tour, vehicle_break: &VehicleBreak) -> Result<TimeWindow, String> {
    match &vehicle_break.time {
        VehicleBreakTime::TimeWindow(tw) => Ok(parse_time_window(tw)),
        VehicleBreakTime::TimeOffset(offset) => {
            if offset.len() != 2 {
                return Err(format!("invalid offset break for tour: '{}'", tour.vehicle_id));
            }

            let departure = tour
                .stops
                .first()
                .map(|stop| parse_time(&stop.time.departure))
                .ok_or_else(|| format!("cannot get departure time for tour: '{}'", tour.vehicle_id))?;
            Ok(TimeWindow::new(departure + *offset.first().unwrap(), departure + *offset.last().unwrap()))
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
