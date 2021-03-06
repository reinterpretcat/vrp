#[cfg(test)]
#[path = "../../tests/unit/checker/routing_test.rs"]
mod routing_test;

use super::*;
use crate::format::CoordIndex;
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
    let matrices = get_matrices(context)?;
    let matrix_size = get_matrix_size(matrices);
    let profile_index = get_profile_index(context, matrices)?;
    let coord_index = CoordIndex::new(&context.problem);
    let skip_distance_check = skip_distance_check(&context.solution);

    context.solution.tours.iter().try_for_each::<_, Result<_, String>>(|tour| {
        let profile = &context.get_vehicle(&tour.vehicle_id)?.profile;
        let matrix = profile_index
            .get(profile.matrix.as_str())
            .and_then(|idx| matrices.get(*idx))
            .ok_or(format!("cannot get matrix for '{}' profile", profile.matrix))?;
        let time_offset =
            parse_time(&tour.stops.first().ok_or_else(|| "empty tour".to_string())?.time.departure) as i64;

        let (departure_time, total_distance) = tour.stops.windows(2).enumerate().try_fold::<_, _, Result<_, String>>(
            (time_offset, 0),
            |(time, total_distance), (leg_idx, stops)| {
                let (from, to) = match stops {
                    [from, to] => (from, to),
                    _ => unreachable!(),
                };

                let from_idx = get_location_index(&from.location, &coord_index)?;
                let to_idx = get_location_index(&to.location, &coord_index)?;
                let matrix_idx = from_idx * matrix_size + to_idx;

                let distance = get_matrix_value(matrix_idx, &matrix.distances)?;
                let duration = get_matrix_value(matrix_idx, &matrix.travel_times)?;
                let duration = (duration as f64 * profile.scale.unwrap_or(1.)) as i64;

                let time = time + duration;
                let total_distance = total_distance + distance;

                check_stop_statistic(time, total_distance, leg_idx + 1, to, tour, skip_distance_check)?;

                Ok((parse_time(&to.time.departure) as i64, to.distance))
            },
        )?;

        check_tour_statistic(departure_time, total_distance, time_offset, tour, skip_distance_check)
    })?;

    check_solution_statistic(&context.solution)
}

fn check_stop_statistic(
    time: i64,
    total_distance: i64,
    stop_idx: usize,
    to: &Stop,
    tour: &Tour,
    skip_distance_check: bool,
) -> Result<(), String> {
    if (time - parse_time(&to.time.arrival) as i64).abs() > 1 {
        return Err(format!(
            "arrival time mismatch for {} stop in the tour: {}, expected: '{}', got: '{}'",
            stop_idx,
            tour.vehicle_id,
            format_time(time as f64),
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

fn get_matrices(context: &CheckerContext) -> Result<&Vec<Matrix>, String> {
    let matrices = context.matrices.as_ref().unwrap();

    if matrices.iter().any(|matrix| matrix.timestamp.is_some()) {
        return Err("not implemented: time aware routing check".to_string());
    }

    Ok(matrices)
}

fn get_matrix_size(matrices: &[Matrix]) -> usize {
    (matrices.first().unwrap().travel_times.len() as f64).sqrt().round() as usize
}

fn get_matrix_value(idx: usize, matrix_values: &[i64]) -> Result<i64, String> {
    matrix_values
        .get(idx)
        .cloned()
        .ok_or_else(|| format!("attempt to get value out of bounds: {} vs {}", idx, matrix_values.len()))
}

fn get_profile_index<'a>(context: &'a CheckerContext, matrices: &[Matrix]) -> Result<HashMap<&'a str, usize>, String> {
    let profiles = context.problem.fleet.profiles.len();
    if profiles != matrices.len() {
        return Err(format!(
            "precondition failed: amount of matrices supplied ({}) does not match profile specified ({})",
            matrices.len(),
            profiles,
        ));
    }

    Ok(context
        .problem
        .fleet
        .profiles
        .iter()
        .enumerate()
        .map(|(idx, profile)| (profile.name.as_str(), idx))
        .collect::<HashMap<_, _>>())
}

fn get_location_index(location: &Location, coord_index: &CoordIndex) -> Result<usize, String> {
    coord_index.get_by_loc(location).ok_or_else(|| format!("cannot find coordinate in coord index: {:?}", location))
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
