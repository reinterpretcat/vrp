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
                MinimizeDistance => acc.entry("minimize-distance"),
                MinimizeDuration => acc.entry("minimize-duration"),
                MinimizeTours => acc.entry("minimize-tours"),
                MaximizeTours => acc.entry("maximize-tours"),
                MaximizeValue { .. } => acc.entry("maximize-value"),
                MinimizeUnassignedJobs { .. } => acc.entry("minimize-unassigned"),
                MinimizeArrivalTime => acc.entry("minimize-arrival-time"),
                BalanceMaxLoad { .. } => acc.entry("balance-max-load"),
                BalanceActivities { .. } => acc.entry("balance-activities"),
                BalanceDistance { .. } => acc.entry("balance-distance"),
                BalanceDuration { .. } => acc.entry("balance-duration"),
                CompactTour { .. } => acc.entry("compact-tour"),
                TourOrder => acc.entry("tour-order"),
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
    let no_min_cost =
        !objectives.iter().any(|objective| matches!(objective, MinimizeCost | MinimizeDistance | MinimizeDuration));

    if no_min_cost {
        Err(FormatError::new(
            "E1602".to_string(),
            "missing one of cost objectives".to_string(),
            "specify 'minimize-cost', 'minimize-duration' or 'minimize-distance' objective".to_string(),
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
    let has_no_jobs_with_value = !ctx.problem.plan.jobs.iter().filter_map(|job| job.value).any(|value| value > 0.);

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

/// Checks that order objective can be specified only when job with order is used.
fn check_e1604_no_jobs_with_order_objective(
    ctx: &ValidationContext,
    objectives: &[&Objective],
) -> Result<(), FormatError> {
    let has_order_objective = objectives.iter().any(|objective| matches!(objective, TourOrder { .. }));
    let has_no_jobs_with_order =
        !ctx.problem.plan.jobs.iter().flat_map(get_job_tasks).filter_map(|job| job.order).any(|value| value > 0);

    if has_order_objective && has_no_jobs_with_order {
        Err(FormatError::new(
            "E1604".to_string(),
            "redundant tour order objective".to_string(),
            "specify at least one job with non-zero order or delete 'tour-order' objective".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn check_e1605_check_positive_value_and_order(ctx: &ValidationContext) -> Result<(), FormatError> {
    let job_ids = ctx
        .problem
        .plan
        .jobs
        .iter()
        .filter(|job| {
            let has_invalid_order = get_job_tasks(job).filter_map(|task| task.order).any(|value| value < 1);
            let has_invalid_value = job.value.map_or(false, |v| v < 1.);

            has_invalid_order || has_invalid_value
        })
        .map(|job| job.id.as_str())
        .collect::<Vec<_>>();

    if job_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1605".to_string(),
            "value or order of a job should be greater than zero".to_string(),
            format!("change value or order of jobs to be greater than zero: '{}'", job_ids.join(", ")),
        ))
    }
}

/// Checks that only one cost objective is specified.
fn check_e1606_check_multiple_cost_objectives(objectives: &[&Objective]) -> Result<(), FormatError> {
    let cost_objectives = objectives
        .iter()
        .filter(|objective| matches!(objective, MinimizeCost | MinimizeDistance | MinimizeDuration))
        .count();

    if cost_objectives > 1 {
        Err(FormatError::new(
            "E1606".to_string(),
            "multiple cost objectives specified".to_string(),
            format!("keep only one cost objective: was specified: '{cost_objectives}'"),
        ))
    } else {
        Ok(())
    }
}

/// Checks that value objective is specified when some jobs have value property set.
fn check_e1607_jobs_with_value_but_no_objective(
    ctx: &ValidationContext,
    objectives: &[&Objective],
) -> Result<(), FormatError> {
    if objectives.is_empty() {
        return Ok(());
    }

    let has_no_value_objective = !objectives.iter().any(|objective| matches!(objective, MaximizeValue { .. }));
    let has_jobs_with_vlue = ctx.problem.plan.jobs.iter().filter_map(|job| job.value).any(|value| value > 0.);

    if has_no_value_objective && has_jobs_with_vlue {
        Err(FormatError::new(
            "E1607".to_string(),
            "missing value objective".to_string(),
            "specify 'maximize-value' objective, remove objectives property or remove value property from jobs"
                .to_string(),
        ))
    } else {
        Ok(())
    }
}

fn get_objectives<'a>(ctx: &'a ValidationContext) -> Option<Vec<&'a Objective>> {
    ctx.problem.objectives.as_ref().map(|objectives| objectives.iter().flatten().collect())
}

pub fn validate_objectives(ctx: &ValidationContext) -> Result<(), MultiFormatError> {
    if let Some(objectives) = get_objectives(ctx) {
        combine_error_results(&[
            check_e1600_empty_objective(&objectives),
            check_e1601_duplicate_objectives(&objectives),
            check_e1602_no_cost_objective(&objectives),
            check_e1603_no_jobs_with_value_objective(ctx, &objectives),
            check_e1604_no_jobs_with_order_objective(ctx, &objectives),
            check_e1605_check_positive_value_and_order(ctx),
            check_e1606_check_multiple_cost_objectives(&objectives),
            check_e1607_jobs_with_value_but_no_objective(ctx, &objectives),
        ])
        .map_err(|errors| errors.into())
    } else {
        Ok(())
    }
}
