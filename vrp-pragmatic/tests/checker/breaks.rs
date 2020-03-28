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
    if let &[from, to] = &leg {
        if let Some(activity_type) = context.get_activity_type(tour, stop, to).ok() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format_time;

    parameterized_test! {can_check_breaks, (break_times, expected_result), {
        can_check_breaks_impl(break_times, expected_result);
    }}

    can_check_breaks! {
        case01: (VehicleBreakTime::TimeOffset(vec![2., 5.]), Ok(())),
        case02: (VehicleBreakTime::TimeOffset(vec![3., 6.]), Ok(())),
        case03: (VehicleBreakTime::TimeOffset(vec![0., 1.]),  Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),
        case04: (VehicleBreakTime::TimeOffset(vec![7., 10.]), Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),

        case05: (VehicleBreakTime::TimeWindow(vec![format_time(2.), format_time(5.)]), Ok(())),
        case06: (VehicleBreakTime::TimeWindow(vec![format_time(3.), format_time(6.)]), Ok(())),
        case07: (VehicleBreakTime::TimeWindow(vec![format_time(0.), format_time(1.)]),
                 Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),
        case08: (VehicleBreakTime::TimeWindow(vec![format_time(7.), format_time(10.)]),
                 Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),
    }

    fn can_check_breaks_impl(break_times: VehicleBreakTime, expected_result: Result<(), String>) {
        let problem = Problem {
            plan: Plan {
                jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![2., 0.])],
                relations: None,
            },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    shifts: vec![VehicleShift {
                        start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                        end: Some(VehiclePlace {
                            time: format_time(1000.).to_string(),
                            location: vec![0., 0.].to_loc(),
                        }),
                        breaks: Some(vec![VehicleBreak { time: break_times, duration: 0.0, locations: None }]),
                        reloads: None,
                    }],
                    capacity: vec![5],
                    ..create_default_vehicle_type()
                }],
                profiles: create_default_profiles(),
            },
            ..create_empty_problem()
        };
        let matrix = create_matrix_from_problem(&problem);
        let solution = Solution {
            statistic: Statistic {
                cost: 22.,
                distance: 4,
                duration: 8,
                times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 2 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        5,
                    ),
                    Stop {
                        location: vec![2., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:03Z".to_string(),
                            departure: "1970-01-01T00:00:06Z".to_string(),
                        },
                        distance: 2,
                        load: vec![0],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: None,
                                time: None,
                                job_tag: None,
                            },
                            Activity {
                                job_id: "break".to_string(),
                                activity_type: "break".to_string(),
                                location: None,
                                time: None,
                                job_tag: None,
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:08Z", "1970-01-01T00:00:08Z"),
                        4,
                    ),
                ],
                statistic: Statistic {
                    cost: 22.,
                    distance: 4,
                    duration: 8,
                    times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            unassigned: vec![],
            extras: None,
        };

        let result = check_breaks(&CheckerContext::new(problem, Some(vec![matrix]), solution));

        assert_eq!(result, expected_result);
    }
}
