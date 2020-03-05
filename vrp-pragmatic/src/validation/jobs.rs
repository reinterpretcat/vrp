use super::*;
use crate::extensions::MultiDimensionalCapacity;
use std::collections::HashSet;

/// Checks that plan has no jobs with duplicate ids (E1000).
fn check_e1000_no_jobs_with_duplicate_ids(ctx: ValidationContext) -> Result<(), String> {
    let mut jobs = HashSet::<_>::default();
    let duplicated_ids = ctx
        .jobs()
        .map(|job| &job.id)
        .filter_map(move |id| if jobs.insert(id) { None } else { Some(id.clone()) })
        .collect::<HashSet<_>>();

    if duplicated_ids.is_empty() {
        Ok(())
    } else {
        Err(format!("E1000: Duplicated job ids: {:?}", duplicated_ids))
    }
}

/// Checks that sum of pickup/delivery demand should be equal (E1001).
fn check_e1001_multiple_pickups_deliveries_demand(ctx: ValidationContext) -> Result<(), String> {
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
        Err(format!("E1001: Invalid demand in jobs with {}", ids.join(",")))
    }
}
