#[cfg(test)]
#[path = "../../tests/unit/checker/breaks_test.rs"]
mod breaks_test;

use super::*;

/// Checks that breaks are properly assigned.
pub fn check_breaks(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        let vehicle_shift = context.get_vehicle_shift(tour)?;
        let actual_break_count = tour.stops.iter().try_fold(0, |acc, stop| {
            stop.activities.windows(2).flat_map(|leg| as_leg_with_break(context, tour, stop, leg)).try_fold(
                acc,
                |acc, (from, to, vehicle_break)| {
                    // check time
                    let visit_time = get_time_window(stop, to);
                    let break_time_window = get_break_time_window(tour, &vehicle_break)?;
                    if !visit_time.intersects(&break_time_window) {
                        return Err(format!(
                            "Break visit time '{:?}' is invalid: expected is in '{:?}'",
                            visit_time, break_time_window
                        ));
                    }

                    // check location
                    let actual_location = get_location(stop, to);
                    match &vehicle_break.locations {
                        Some(locations) => {
                            let is_correct =
                                locations.iter().any(|location| same_locations(&actual_location, location));

                            if !is_correct {
                                return Err(format!(
                                    "Break location '{:?}' is invalid: expected one of '{:?}'",
                                    actual_location, locations
                                ));
                            }
                        }
                        None => {
                            let prev_location = get_location(stop, from);
                            if !same_locations(&prev_location, &actual_location) {
                                return Err(format!(
                                    "Break location '{:?}' is invalid: expected previous activity location '{:?}'",
                                    actual_location, prev_location
                                ));
                            }
                        }
                    }

                    Ok(acc + 1)
                },
            )
        })?;

        let arrival = tour
            .stops
            .last()
            .map(|stop| parse_time(&stop.time.arrival))
            .ok_or_else(|| format!("Cannot get arrival for tour '{}'", tour.vehicle_id))?;

        let expected_break_count =
            vehicle_shift.breaks.iter().flat_map(|breaks| breaks.iter()).fold(0, |acc, vehicle_break| {
                let break_time = get_break_time_window(tour, vehicle_break).expect("Cannot get break time windows");

                if break_time.start < arrival {
                    acc + 1
                } else {
                    acc
                }
            });

        if expected_break_count != actual_break_count {
            Err(format!(
                "Amount of breaks does not match, expected: '{}', got '{}'",
                expected_break_count, actual_break_count
            ))
        } else {
            Ok(())
        }
    })
}

fn as_leg_with_break<'a>(
    context: &CheckerContext,
    tour: &Tour,
    stop: &Stop,
    leg: &'a [Activity],
) -> Option<(&'a Activity, &'a Activity, VehicleBreak)> {
    if let [from, to] = leg {
        if let Ok(activity_type) = context.get_activity_type(tour, stop, to) {
            if let ActivityType::Break(vehicle_break) = activity_type {
                return Some((from, to, vehicle_break));
            }
        }
    }
    None
}

fn get_break_time_window(tour: &Tour, vehicle_break: &VehicleBreak) -> Result<TimeWindow, String> {
    match &vehicle_break.time {
        VehicleBreakTime::TimeWindow(tw) => Ok(parse_time_window(tw)),
        VehicleBreakTime::TimeOffset(offset) => {
            if offset.len() != 2 {
                return Err(format!("Invalid offset break for tour: '{}'", tour.vehicle_id));
            }

            let departure = tour
                .stops
                .first()
                .map(|stop| parse_time(&stop.time.departure))
                .ok_or_else(|| format!("Cannot get departure time for tour: '{}'", tour.vehicle_id))?;
            Ok(TimeWindow::new(departure + *offset.first().unwrap(), departure + *offset.last().unwrap()))
        }
    }
}
