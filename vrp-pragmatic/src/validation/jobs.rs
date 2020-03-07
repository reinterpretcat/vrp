use super::*;
use crate::extensions::MultiDimensionalCapacity;

/// Checks that plan has no jobs with duplicate ids.
fn check_e1000_no_jobs_with_duplicate_ids(ctx: &ValidationContext) -> Result<(), String> {
    get_duplicates(ctx.jobs().map(|job| &job.id))
        .map_or(Ok(()), |ids| Err(format!("E1000: Duplicated job ids: {}", ids.join(", "))))
}

/// Checks that sum of pickup/delivery demand should be equal.
fn check_e1001_multiple_pickups_deliveries_demand(ctx: &ValidationContext) -> Result<(), String> {
    let has_tasks = |tasks: &Option<Vec<JobTask>>| tasks.as_ref().map_or(false, |tasks| tasks.len() > 0);
    let get_demand = |tasks: &Option<Vec<JobTask>>| {
        if let Some(tasks) = tasks {
            tasks.iter().map(|task| MultiDimensionalCapacity::new(task.demand.clone())).sum()
        } else {
            MultiDimensionalCapacity::default()
        }
    };

    let ids = ctx
        .jobs()
        .filter(|job| has_tasks(&job.pickups) && has_tasks(&job.deliveries))
        .filter(|job| get_demand(&job.pickups) - get_demand(&job.deliveries) != MultiDimensionalCapacity::default())
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(format!("E1001: Invalid demand in jobs: {}", ids.join(", ")))
    }
}

/// Checks that job's time windows are correct.
fn check_e1002_time_window_correctness(ctx: &ValidationContext) -> Result<(), String> {
    let has_invalid_tws = |tasks: &Option<Vec<JobTask>>| {
        tasks.as_ref().map_or(false, |tasks| {
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
        Err(format!("E1002: Invalid time windows in jobs: {}", ids.join(", ")))
    }
}

/// Validates jobs from the plan.
pub fn validate_jobs(ctx: &ValidationContext) -> Result<(), Vec<String>> {
    let errors = check_e1000_no_jobs_with_duplicate_ids(ctx)
        .err()
        .iter()
        .cloned()
        .chain(check_e1001_multiple_pickups_deliveries_demand(ctx).err().iter().cloned())
        .chain(check_e1002_time_window_correctness(ctx).err().iter().cloned())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
