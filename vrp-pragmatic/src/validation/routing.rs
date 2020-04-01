use super::*;

/// Checks that objective is not empty when specified.
fn check_e1012_duplicated_profiles(ctx: &ValidationContext) -> Result<(), FormatError> {
    get_duplicates(ctx.problem.fleet.profiles.iter().map(|p| &p.name)).map_or(Ok(()), |names| {
        Err(FormatError::new(
            "E1012".to_string(),
            "duplicated profile names".to_string(),
            format!("remove duplicates of profiles with the names: '{}'", names.join(", ")),
        ))
    })
}

/// Validates profiles from the fleet.
pub fn validate_profiles(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    combine_error_results(&[
        check_e1012_duplicated_profiles(ctx), //
    ])
}
