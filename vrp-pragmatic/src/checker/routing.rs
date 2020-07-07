#[cfg(test)]
#[path = "../../tests/unit/checker/routing_test.rs"]
mod routing_test;

use super::*;
use crate::format::CoordIndex;
use crate::format_time;

/// Checks that matrix routing information is used properly.
pub fn check_routing(context: &CheckerContext) -> Result<(), String> {
    if context.matrices.as_ref().map_or(true, |m| m.is_empty()) {
        return Ok(());
    }
    let matrices = get_matrices(context)?;
    let matrix_size = get_matrix_size(matrices);
    let profile_index = get_profile_index(context, matrices)?;
    let coord_index = CoordIndex::new(&context.problem);

    context.solution.tours.iter().try_for_each::<_, Result<_, String>>(|tour| {
        let profile = &context.get_vehicle(tour.vehicle_id.as_str())?.profile;
        let matrix = profile_index
            .get(profile.as_str())
            .and_then(|idx| matrices.get(*idx))
            .ok_or(format!("cannot get matrix for '{}' profile", profile))?;
        let time_offset = parse_time(&tour.stops.first().ok_or_else(|| "empty tour".to_string())?.time.departure) as i64;

        tour.stops.windows(2).try_fold((time_offset, 0), |(time, total_distance), stops| {
            let (from, to) = match &stops {
                &[from, to] => (from, to),
                _ => unreachable!(),
            };

            let from_idx = get_location_index(&from.location, &coord_index)?;
            let to_idx = get_location_index(&to.location, &coord_index)?;
            let matrix_idx = from_idx * matrix_size + to_idx;

            let distance = get_matrix_value(matrix_idx, &matrix.distances)?;
            let duration = get_matrix_value(matrix_idx, &matrix.distances)?;

            let time = time + duration;
            let total_distance = total_distance + distance as i32;

            if time != parse_time(&to.time.arrival) as i64 {
                return Err(format!(
                    "arrival time mismatch for tour: {}, expected: '{}', got: '{}'",
                    tour.vehicle_id,
                    format_time(time as f64),
                    to.time.arrival
                ));
            }

            if total_distance != to.distance {
                return Err(format!(
                    "distance mismatch for tour: {}, expected: '{}', got: '{}'",
                    tour.vehicle_id, to.distance, total_distance
                ));
            }

            Ok((parse_time(&to.time.departure) as i64, total_distance))
        })?;

        Ok(())
    })?;

    Ok(())
}

fn get_matrices(context: &CheckerContext) -> Result<&Vec<Matrix>, String> {
    let matrices = context.matrices.as_ref().unwrap();

    if matrices.iter().any(|matrix| matrix.timestamp.is_some()) {
        return Err("not implemented: time aware routing check".to_string());
    }

    Ok(matrices)
}

fn get_matrix_size(matrices: &Vec<Matrix>) -> usize {
    (matrices.first().unwrap().travel_times.len() as f64).sqrt().round() as usize
}

fn get_matrix_value(idx: usize, matrix_values: &Vec<i64>) -> Result<i64, String> {
    matrix_values
        .get(idx)
        .cloned()
        .ok_or_else(|| format!("attempt to get value out of bounds: {} vs {}", idx, matrix_values.len()))
}

fn get_profile_index<'a>(
    context: &'a CheckerContext,
    matrices: &Vec<Matrix>,
) -> Result<HashMap<&'a str, usize>, String> {
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
