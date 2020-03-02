use super::*;

/// Checks that breaks are properly assigned.
pub fn check_breaks(context: &CheckerContext) -> Result<(), String> {
    context.solution.tours.iter().try_for_each(|tour| {
        tour.stops.iter().try_for_each(|stop| {
            stop.activities.windows(2).flat_map(|leg| as_leg_with_break(context, tour, stop, leg)).try_for_each(
                |(from, to, vehicle_break)| {
                    // check time
                    let visit_time = get_time_window(stop, to);
                    match &vehicle_break.times {
                        VehicleBreakTime::TimeWindows(windows) => {
                            let times = parse_time_windows(windows);
                            let is_proper = times.iter().any(|tw| tw.intersects(&visit_time));

                            if !is_proper {
                                return Err(format!(
                                    "Break visit time '{:?}' is invalid: expected is tws in '{:?}'",
                                    visit_time, times
                                ));
                            }
                        }
                        VehicleBreakTime::TimeOffset(offset) => {
                            if offset.len() != 2 {
                                return Err(format!("Invalid offset break for tour: '{}'", tour.vehicle_id));
                            }

                            let departure =
                                tour.stops.first().map(|stop| parse_time(&stop.time.departure)).ok_or_else(|| {
                                    format!("Cannot get departure time for tour: '{}'", tour.vehicle_id)
                                })?;
                            let time = TimeWindow::new(
                                departure + *offset.first().unwrap(),
                                departure + *offset.last().unwrap(),
                            );

                            if !visit_time.intersects(&time) {
                                return Err(format!(
                                    "Break visit time '{:?}' is invalid: expected is offset in '{:?}'",
                                    visit_time, time
                                ));
                            }
                        }
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

                    Ok(())
                },
            )
        })
    })
}

fn as_leg_with_break<'a>(
    context: &CheckerContext,
    tour: &Tour,
    stop: &Stop,
    leg: &'a [Activity],
) -> Option<(&'a Activity, &'a Activity, VehicleBreak)> {
    if let &[from, to] = &leg {
        if let Some(activity_type) = context.get_activity_type(tour, stop, to).ok() {
            if let ActivityType::Break(vehicle_break) = activity_type {
                return Some((from, to, vehicle_break));
            }
        }
    }
    None
}
