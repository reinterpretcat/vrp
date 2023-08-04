#[cfg(test)]
#[path = "../../tests/unit/checker/routing_test.rs"]
mod routing_test;

use super::*;
use crate::format_time;
use crate::utils::combine_error_results;

/// Checks that matrix routing information is used properly.
pub fn check_routing(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_routing_rules(context)])
}

fn check_routing_rules(context: &CheckerContext) -> Result<(), GenericError> {
    if context.matrices.as_ref().map_or(true, |m| m.is_empty()) {
        return Ok(());
    }
    let skip_distance_check = skip_distance_check(&context.solution);

    context.solution.tours.iter().try_for_each::<_, Result<_, GenericError>>(|tour| {
        let profile = context.get_vehicle_profile(&tour.vehicle_id)?;

        let get_matrix_data = |from: &PointStop, to: &PointStop| -> Result<(i64, i64), GenericError> {
            let from_idx = context.get_location_index(&from.location)?;
            let to_idx = context.get_location_index(&to.location)?;
            context.get_matrix_data(&profile, from_idx, to_idx)
        };

        let first_stop = tour.stops.first().ok_or_else(|| "empty tour".to_string())?;
        let first_activity =
            first_stop.activities().first().ok_or_else(|| "no activities in first stop".to_string())?;
        let time_offset = parse_time(
            first_activity
                .time
                .as_ref()
                .map(|interval| &interval.end)
                .unwrap_or_else(|| &first_stop.schedule().departure),
        ) as i64;

        let (departure_time, total_distance) =
            tour.stops.windows(2).enumerate().try_fold::<_, _, Result<_, GenericError>>(
                (parse_time(&first_stop.schedule().departure) as i64, 0),
                |(arrival_time, total_distance), (leg_idx, stops)| {
                    let (from, to) = match stops {
                        [from, to] => (from, to),
                        _ => unreachable!(),
                    };

                    let (distance, duration, to_distance) = match (from, to) {
                        (Stop::Point(from), Stop::Point(to)) => {
                            let (distance, duration) = get_matrix_data(from, to)?;
                            (distance, duration, to.distance)
                        }
                        (prev, Stop::Transit(transit)) => {
                            let prev_departure = parse_time(&prev.schedule().departure);
                            let next_arrival = parse_time(&transit.time.arrival);
                            // NOTE an edge case: duration of break will be counted in transit stop
                            let duration = if next_arrival == prev_departure {
                                0.
                            } else {
                                parse_time(&transit.time.departure) - next_arrival
                            };
                            (0_i64, duration as i64, total_distance)
                        }
                        (Stop::Transit(_), Stop::Point(to)) => {
                            assert!(leg_idx > 0);
                            let from = tour
                                .stops
                                .get(leg_idx - 1)
                                .unwrap()
                                .as_point()
                                .expect("two consistent transit stops are not supported");
                            let (distance, duration) = get_matrix_data(from, to)?;
                            (distance, duration, to.distance)
                        }
                    };

                    let arrival_time = arrival_time + duration;
                    let total_distance = total_distance + distance;

                    check_stop_statistic(
                        arrival_time,
                        total_distance,
                        to.schedule(),
                        to_distance,
                        leg_idx + 1,
                        tour,
                        skip_distance_check,
                    )?;

                    Ok((parse_time(&to.schedule().departure) as i64, to_distance))
                },
            )?;

        check_tour_statistic(departure_time, total_distance, time_offset, tour, skip_distance_check)
    })?;

    check_solution_statistic(&context.solution)
}

fn check_stop_statistic(
    arrival_time: i64,
    total_distance: i64,
    schedule: &Schedule,
    distance: i64,
    stop_idx: usize,
    tour: &Tour,
    skip_distance_check: bool,
) -> Result<(), GenericError> {
    if (arrival_time - parse_time(&schedule.arrival) as i64).abs() > 1 {
        return Err(format!(
            "arrival time mismatch for {stop_idx} stop in the tour: {}, expected: '{}', got: '{}'",
            tour.vehicle_id,
            format_time(arrival_time as f64),
            schedule.arrival
        )
        .into());
    }

    if !skip_distance_check && (total_distance - distance).abs() > 1 {
        return Err(format!(
            "distance mismatch for {stop_idx} stop in the tour: {}, expected: '{total_distance}', got: '{distance}'",
            tour.vehicle_id
        )
        .into());
    }

    Ok(())
}

fn check_tour_statistic(
    departure_time: i64,
    total_distance: i64,
    time_offset: i64,
    tour: &Tour,
    skip_distance_check: bool,
) -> Result<(), GenericError> {
    if !skip_distance_check && (total_distance - tour.statistic.distance).abs() > 1 {
        return Err(format!(
            "distance mismatch for tour statistic: {}, expected: '{}', got: '{}'",
            tour.vehicle_id, total_distance, tour.statistic.distance,
        )
        .into());
    }

    let total_duration = departure_time - time_offset;
    if (total_duration - tour.statistic.duration).abs() > 1 {
        return Err(format!(
            "duration mismatch for tour statistic: {}, expected: '{}', got: '{}'",
            tour.vehicle_id, total_duration, tour.statistic.duration,
        )
        .into());
    }

    Ok(())
}

fn check_solution_statistic(solution: &Solution) -> Result<(), GenericError> {
    let statistic = solution.tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

    // NOTE cost should be ignored due to floating point issues
    if statistic.duration != solution.statistic.duration || statistic.distance != solution.statistic.distance {
        Err(format!("solution statistic mismatch, expected: '{:?}', got: '{:?}'", statistic, solution.statistic).into())
    } else {
        Ok(())
    }
}

/// A workaround method for hre format output where distance is not defined.
fn skip_distance_check(solution: &Solution) -> bool {
    let skip_distance_check = solution
        .tours
        .iter()
        .flat_map(|tour| tour.stops.iter())
        .filter_map(|stop| stop.as_point())
        .all(|stop| stop.distance == 0);

    if skip_distance_check {
        // TODO use logging lib instead of println
        println!("all stop distances are zeros: no distance check will be performed");
    }

    skip_distance_check
}
