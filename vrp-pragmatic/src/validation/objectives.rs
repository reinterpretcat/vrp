#[cfg(test)]
#[path = "../../tests/unit/validation/objectives_test.rs"]
mod objectives_test;

use super::*;
use crate::format::problem::Objective::*;
use crate::utils::combine_error_results;

/// Checks that objective is not empty when specified.
fn check_e1600_empty_objective(objectives: &[&Objective]) -> Result<(), FormatError> {
    if objectives.is_empty() {
        Err(FormatError::new(
            "E1600".to_string(),
            "an empty objective specified".to_string(),
            "remove objectives property completely to use default".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that each objective type specified only once.
fn check_e1601_duplicate_objectives(objectives: &[&Objective]) -> Result<(), FormatError> {
    let mut duplicates = objectives
        .iter()
        .fold(HashMap::new(), |mut acc, objective| {
            match objective {
                MinimizeCost => acc.entry("minimize-cost"),
                MinimizeTours => acc.entry("minimize-tours"),
                MaximizeTours => acc.entry("maximize-tours"),
                MaximizeValue { .. } => acc.entry("maximize-value"),
                MinimizeUnassignedJobs { .. } => acc.entry("minimize-unassigned"),
                BalanceMaxLoad { .. } => acc.entry("balance-max-load"),
                BalanceActivities { .. } => acc.entry("balance-activities"),
                BalanceDistance { .. } => acc.entry("balance-distance"),
                BalanceDuration { .. } => acc.entry("balance-duration"),
            }
            .and_modify(|count| *count += 1)
            .or_insert(1_usize);

            acc
        })
        .iter()
        .filter_map(|(name, count)| if *count > 1 { Some((*name).to_string()) } else { None })
        .collect::<Vec<_>>();

    duplicates.sort();

    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1601".to_string(),
            "duplicate objective specified".to_string(),
            "remove duplicate objectives".to_string(),
        ))
    }
}

/// Checks that cost objective is specified.
fn check_e1602_no_cost_objective(objectives: &[&Objective]) -> Result<(), FormatError> {
    let no_min_cost = objectives.iter().find(|objective| matches!(objective, MinimizeCost)).is_none();

    if no_min_cost {
        Err(FormatError::new(
            "E1602".to_string(),
            "missing cost objective".to_string(),
            "specify 'minimize-cost' objective".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that value objective can be specified only when job with value is used.
fn check_e1603_no_jobs_with_value_objective(
    ctx: &ValidationContext,
    objectives: &[&Objective],
) -> Result<(), FormatError> {
    let has_value_objective = objectives.iter().any(|objective| matches!(objective, MaximizeValue { .. }));
    let has_no_jobs_with_value =
        ctx.problem.plan.jobs.iter().filter_map(|job| job.value).find(|value| *value > 0.).is_none();

    if has_value_objective && has_no_jobs_with_value {
        Err(FormatError::new(
            "E1603".to_string(),
            "redundant value objective".to_string(),
            "specify at least one non-zero valued job or delete 'maximize-value' objective".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn get_objectives<'a>(ctx: &'a ValidationContext) -> Option<Vec<&'a Objective>> {
    ctx.problem.objectives.as_ref().map(|objectives| objectives.iter().flatten().collect())
}

pub fn validate_objectives(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    if let Some(objectives) = get_objectives(ctx) {
        combine_error_results(&[
            check_e1600_empty_objective(&objectives),
            check_e1601_duplicate_objectives(&objectives),
            check_e1602_no_cost_objective(&objectives),
            check_e1603_no_jobs_with_value_objective(ctx, &objectives),
        ])
    } else {
        Ok(())
    }
}
