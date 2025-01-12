#[cfg(test)]
#[path = "../../tests/unit/validation/jobs_test.rs"]
mod jobs_test;

use super::*;
use crate::utils::combine_error_results;
use vrp_core::models::common::MultiDimLoad;

/// Checks that plan has no jobs with duplicate ids.
fn check_e1100_no_jobs_with_duplicate_ids(ctx: &ValidationContext) -> Result<(), FormatError> {
    get_duplicates(ctx.jobs().map(|job| &job.id)).map_or(Ok(()), |ids| {
        Err(FormatError::new(
            "E1100".to_string(),
            "duplicated job ids".to_string(),
            format!("remove duplicated jobs with for the ids: '{}'", ids.join(", ")),
        ))
    })
}

/// Checks that jobs have proper demand.
fn check_e1101_correct_job_types_demand(ctx: &ValidationContext) -> Result<(), FormatError> {
    let ids = ctx
        .jobs()
        .filter(|job| {
            job.pickups
                .iter()
                .chain(job.deliveries.iter())
                .chain(job.replacements.iter())
                .flat_map(|tasks| tasks.iter())
                .any(|task| task.demand.is_none())
                || job.services.iter().flat_map(|tasks| tasks.iter()).any(|task| task.demand.is_some())
        })
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1101".to_string(),
            "invalid job task demand".to_string(),
            format!("correct demand based on job task type for jobs: '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that sum of pickup/delivery demand should be equal.
fn check_e1102_multiple_pickups_deliveries_demand(ctx: &ValidationContext) -> Result<(), FormatError> {
    let has_tasks = |tasks: &Option<Vec<JobTask>>| tasks.as_ref().is_some_and(|tasks| !tasks.is_empty());
    let get_demand = |tasks: &Option<Vec<JobTask>>| {
        if let Some(tasks) = tasks {
            tasks.iter().map(|task| task.demand.clone().map_or_else(MultiDimLoad::default, MultiDimLoad::new)).sum()
        } else {
            MultiDimLoad::default()
        }
    };

    let ids = ctx
        .jobs()
        .filter(|job| has_tasks(&job.pickups) && has_tasks(&job.deliveries))
        .filter(|job| get_demand(&job.pickups) - get_demand(&job.deliveries) != MultiDimLoad::default())
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1102".to_string(),
            "invalid pickup and delivery demand".to_string(),
            format!("correct demand so that sum of pickups equal to sum of deliveries, jobs: '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that job's time windows are correct.
fn check_e1103_time_window_correctness(ctx: &ValidationContext) -> Result<(), FormatError> {
    let has_invalid_tws = |tasks: &Option<Vec<JobTask>>| {
        tasks.as_ref().is_some_and(|tasks| {
            tasks
                .iter()
                .flat_map(|task| task.places.iter())
                .filter_map(|place| place.times.as_ref())
                .any(|tws| !check_raw_time_windows(tws, false))
        })
    };

    let ids = ctx
        .jobs()
        .filter(|job| has_invalid_tws(&job.pickups) || has_invalid_tws(&job.deliveries))
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1103".to_string(),
            "invalid time windows in jobs".to_string(),
            format!("change job task place time windows so that they don't intersect, jobs: '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that reserved job ids are no used.
fn check_e1104_no_reserved_ids(ctx: &ValidationContext) -> Result<(), FormatError> {
    let ids = ctx.jobs().filter(|job| is_reserved_job_id(&job.id)).map(|job| job.id.clone()).collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1104".to_string(),
            "reserved job id is used".to_string(),
            format!("change job id from reserved: jobs: '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that job has at least one job task.
fn check_e1105_empty_jobs(ctx: &ValidationContext) -> Result<(), FormatError> {
    let ids = ctx.jobs().filter(|job| ctx.tasks(job).is_empty()).map(|job| job.id.clone()).collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1105".to_string(),
            "empty job".to_string(),
            format!("add at least one job task: ids '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that job has no negative duration aka service time.
fn check_e1106_negative_duration(ctx: &ValidationContext) -> Result<(), FormatError> {
    let ids = ctx
        .jobs()
        .filter(|job| {
            ctx.tasks(job)
                .iter()
                .flat_map(|task| task.places.iter().map(|place| place.duration))
                .any(|duration| duration.is_sign_negative())
        })
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1106".to_string(),
            "job has negative duration".to_string(),
            format!("fix negative duration in jobs with ids: '{}'", ids.join(", ")),
        ))
    }
}

/// Checks that job has no negative demand in any of dimensions.
fn check_e1107_negative_demand(ctx: &ValidationContext) -> Result<(), FormatError> {
    let ids = ctx
        .jobs()
        .filter(|job| {
            ctx.tasks(job)
                .iter()
                .any(|task| task.demand.as_ref().is_some_and(|demand| demand.iter().any(|&dim| dim < 0)))
        })
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1107".to_string(),
            "job has negative demand".to_string(),
            format!("fix negative demand in jobs with ids: '{}'", ids.join(", ")),
        ))
    }
}

/// Validates jobs from the plan.
pub fn validate_jobs(ctx: &ValidationContext) -> Result<(), MultiFormatError> {
    combine_error_results(&[
        check_e1100_no_jobs_with_duplicate_ids(ctx),
        check_e1101_correct_job_types_demand(ctx),
        check_e1102_multiple_pickups_deliveries_demand(ctx),
        check_e1103_time_window_correctness(ctx),
        check_e1104_no_reserved_ids(ctx),
        check_e1105_empty_jobs(ctx),
        check_e1106_negative_duration(ctx),
        check_e1107_negative_demand(ctx),
    ])
    .map_err(From::from)
}
