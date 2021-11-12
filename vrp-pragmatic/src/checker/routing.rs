#[cfg(test)]
#[path = "../../tests/unit/checker/routing_test.rs"]
mod routing_test;

use super::*;
use crate::format_time;
use crate::utils::combine_error_results;

/// Checks that matrix routing information is used properly.
pub fn check_routing(context: &CheckerContext) -> Result<(), Vec<String>> {
    combine_error_results(&[check_routing_rules(context)])
}

fn check_routing_rules(context: &CheckerContext) -> Result<(), String> {
    if context.matrices.as_ref().map_or(true, |m| m.is_empty()) {
        return Ok(());
    }
    let skip_distance_check = skip_distance_check(&context.solution);

    context.solution.tours.iter().try_for_each::<_, Result<_, String>>(|tour| {
        let profile = context.get_vehicle_profile(&tour.vehicle_id)?;
        let time_offset =
            parse_time(&tour.stops.first().ok_or_else(|| "empty tour".to_string())?.time.departure) as i64;

        let (departure_time, total_distance) = tour.stops.windows(2).enumerate().try_fold::<_, _, Result<_, String>>(
            (time_offset, 0),
            |(arrival_time, total_distance), (leg_idx, stops)| {
                let (from, to) = match stops {
                    [from, to] => (from, to),
                    _ => unreachable!(),
                };

                let from_idx = context.get_location_index(&from.location)?;
                let to_idx = context.get_location_index(&to.location)?;
                let (distance, duration) = context.get_matrix_data(&profile, from_idx, to_idx)?;

                let arrival_time = arrival_time + duration;
                let total_distance = total_distance + distance;

                check_stop_statistic(arrival_time, total_distance, leg_idx + 1, to, tour, skip_distance_check)?;

                Ok((parse_time(&to.time.departure) as i64, to.distance))
            },
        )?;

        check_tour_statistic(departure_time, total_distance, time_offset, tour, skip_distance_check)
    })?;

    check_solution_statistic(&context.solution)
}

fn check_stop_statistic(
    arrival_time: i64,
    total_distance: i64,
    stop_idx: usize,
    to: &Stop,
    tour: &Tour,
    skip_distance_check: bool,
) -> Result<(), String> {
    if (arrival_time - parse_time(&to.time.arrival) as i64).abs() > 1 {
        return Err(format!(
            "arrival time mismatch for {} stop in the tour: {}, expected: '{}', got: '{}'",
            stop_idx,
            tour.vehicle_id,
            format_time(arrival_time as f64),
            to.time.arrival
        ));
    }

    if !skip_distance_check && (total_distance - to.distance).abs() > 1 {
        return Err(format!(
            "distance mismatch for {} stop in the tour: {}, expected: '{}', got: '{}'",
            stop_idx, tour.vehicle_id, total_distance, to.distance,
        ));
    }

    Ok(())
}

fn check_tour_statistic(
    departure_time: i64,
    total_distance: i64,
    time_offset: i64,
    tour: &Tour,
    skip_distance_check: bool,
) -> Result<(), String> {
    if !skip_distance_check && (total_distance - tour.statistic.distance).abs() > 1 {
        return Err(format!(
            "distance mismatch for tour statistic: {}, expected: '{}', got: '{}'",
            tour.vehicle_id, total_distance, tour.statistic.distance,
        ));
    }

    let dispatch_at_start_correction =
        tour.stops
            .first()
            .and_then(|stop| stop.activities.get(1))
            .and_then(|activity| {
                if activity.activity_type == "dispatch" {
                    Some(
                        activity.time.as_ref().map_or(0, |interval| {
                            parse_time(&interval.end) as i64 - parse_time(&interval.start) as i64
                        }),
                    )
                } else {
                    None
                }
            })
            .unwrap_or(0);

    let total_duration = departure_time - time_offset + dispatch_at_start_correction;
    if (total_duration - tour.statistic.duration).abs() > 1 {
        return Err(format!(
            "duration mismatch for tour statistic: {}, expected: '{}', got: '{}'",
            tour.vehicle_id, total_duration, tour.statistic.duration,
        ));
    }

    Ok(())
}

fn check_solution_statistic(solution: &Solution) -> Result<(), String> {
    let statistic = solution.tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

    // NOTE cost should be ignored due to floating point issues
    if statistic.duration != solution.statistic.duration || statistic.distance != solution.statistic.distance {
        Err(format!("solution statistic mismatch, expected: '{:?}', got: '{:?}'", statistic, solution.statistic))
    } else {
        Ok(())
    }
}

/// A workaround method for hre format output where distance is not defined.
fn skip_distance_check(solution: &Solution) -> bool {
    let skip_distance_check = solution.tours.iter().flat_map(|tour| tour.stops.iter()).all(|stop| stop.distance == 0);

    if skip_distance_check {
        // TODO use logging lib instead of println
        println!("all stop distances are zeros: no distance check will be performed");
    }

    skip_distance_check
}
