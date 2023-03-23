#[cfg(test)]
#[path = "../../tests/unit/validation/routing_test.rs"]
mod routing_test;

use super::*;
use crate::utils::combine_error_results;
use hashbrown::HashSet;

/// Checks that no duplicated profile names specified.
fn check_e1500_duplicated_profiles(ctx: &ValidationContext) -> Result<(), FormatError> {
    get_duplicates(ctx.problem.fleet.profiles.iter().map(|p| &p.name)).map_or(Ok(()), |names| {
        Err(FormatError::new(
            "E1500".to_string(),
            "duplicated profile names".to_string(),
            format!("remove duplicates of profiles with the names: '{}'", names.join(", ")),
        ))
    })
}

/// Checks that profiles collection is not empty.
fn check_e1501_empty_profiles(ctx: &ValidationContext) -> Result<(), FormatError> {
    if ctx.problem.fleet.profiles.is_empty() {
        Err(FormatError::new(
            "E1501".to_string(),
            "empty profile collection".to_string(),
            "specify at least one profile".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that only one type of location is used.
fn check_e1502_no_location_type_mix(_ctx: &ValidationContext, location_types: (bool, bool)) -> Result<(), FormatError> {
    let (has_coordinates, has_indices) = location_types;

    if has_coordinates && has_indices {
        Err(FormatError::new(
            "E1502".to_string(),
            "mixing different location types".to_string(),
            "use either coordinates or indices for all locations".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that routing matrix is supplied when location indices are used.
fn check_e1503_no_matrix_when_indices_used(
    ctx: &ValidationContext,
    location_types: (bool, bool),
) -> Result<(), FormatError> {
    let (_, has_indices) = location_types;

    if has_indices && ctx.matrices.map_or(true, |matrices| matrices.is_empty()) {
        Err(FormatError::new(
            "E1503".to_string(),
            "location indices requires routing matrix to be specified".to_string(),
            "either use coordinates everywhere or specify routing matrix".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that coord index has a proper maximum index for
fn check_e1504_index_size_mismatch(ctx: &ValidationContext) -> Result<(), FormatError> {
    let (max_index, matrix_size, is_correct_index): _ = ctx
        .coord_index
        .max_index()
        .into_iter()
        .zip(
            ctx.matrices
                .and_then(|matrices| matrices.first())
                .map(|matrix| (matrix.distances.len() as f64).sqrt().round() as usize),
        )
        .next()
        .map_or((0_usize, 0_usize, true), |(max_index, matrix_size)| {
            (max_index, matrix_size, max_index + 1 == matrix_size)
        });

    if !is_correct_index {
        Err(FormatError::new(
            "E1504".to_string(),
            "amount of locations does not match matrix dimension".to_string(),
            format!(
                "check matrix size: max location index '{max_index}' + 1 should be equal to matrix size ('{matrix_size}')"
            ),
        ))
    } else {
        Ok(())
    }
}

/// Checks that no duplicated profile names specified.
fn check_e1505_profiles_exist(ctx: &ValidationContext) -> Result<(), FormatError> {
    let known_matrix_profiles = ctx.problem.fleet.profiles.iter().map(|p| p.name.clone()).collect::<HashSet<_>>();

    let unknown_vehicle_profiles = ctx
        .problem
        .fleet
        .vehicles
        .iter()
        .map(|vehicle| vehicle.profile.matrix.clone())
        .chain(ctx.problem.plan.clustering.iter().map(|clustering| match clustering {
            Clustering::Vicinity { profile, .. } => profile.matrix.clone(),
        }))
        .filter(|matrix| !known_matrix_profiles.contains(matrix))
        .collect::<HashSet<_>>();

    if unknown_vehicle_profiles.is_empty() {
        Ok(())
    } else {
        let unknown_profiles = unknown_vehicle_profiles.into_iter().collect::<Vec<_>>();
        Err(FormatError::new(
            "E1505".to_string(),
            "unknown matrix profile name in vehicle or vicinity clustering profile".to_string(),
            format!("ensure that matrix profiles '{}' are defined in profiles", unknown_profiles.join(", ")),
        ))
    }
}

/// Validates routing rules.
pub fn validate_routing(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    let location_types = ctx.coord_index.get_used_types();

    combine_error_results(&[
        check_e1500_duplicated_profiles(ctx),
        check_e1501_empty_profiles(ctx),
        check_e1502_no_location_type_mix(ctx, location_types),
        check_e1503_no_matrix_when_indices_used(ctx, location_types),
        check_e1504_index_size_mismatch(ctx),
        check_e1505_profiles_exist(ctx),
    ])
}
