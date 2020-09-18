#[cfg(test)]
#[path = "../../tests/unit/checker/assignment_test.rs"]
mod assignment_test;

use super::*;
use crate::format::solution::activity_matcher::try_match_job;
use crate::format::{get_coord_index, get_job_index};
use std::collections::HashSet;

/// Checks assignment of jobs and vehicles.
pub fn check_assignment(ctx: &CheckerContext) -> Result<(), String> {
    check_vehicles(ctx)?;
    check_jobs_presence(ctx)?;
    check_jobs_match(ctx)?;

    Ok(())
}

/// Checks that vehicles in each tour are used once per shift and they are known in problem.
fn check_vehicles(ctx: &CheckerContext) -> Result<(), String> {
    let all_vehicles: HashSet<_> = ctx.problem.fleet.vehicles.iter().flat_map(|v| v.vehicle_ids.iter()).collect();
    let mut used_vehicles = HashSet::<(String, usize)>::new();

    ctx.solution.tours.iter().try_for_each(|tour| {
        if !all_vehicles.contains(&tour.vehicle_id) {
            return Err(format!("Used vehicle with unknown id: {}", tour.vehicle_id));
        }

        if !(used_vehicles.insert((tour.vehicle_id.to_string(), tour.shift_index))) {
            Err(format!("Vehicle with '{}' id used more than once for shift {}", tour.vehicle_id, tour.shift_index))
        } else {
            Ok(())
        }
    })?;

    Ok(())
}

/// Checks job task rules.
fn check_jobs_presence(ctx: &CheckerContext) -> Result<(), String> {
    struct JobAssignment {
        pub tour_info: (String, usize),
        pub pickups: Vec<usize>,
        pub deliveries: Vec<usize>,
        pub replacements: Vec<usize>,
        pub services: Vec<usize>,
    }
    let new_assignment = |tour_info: (String, usize)| JobAssignment {
        tour_info,
        pickups: vec![],
        deliveries: vec![],
        replacements: vec![],
        services: vec![],
    };
    let activity_types: HashSet<_> = vec!["pickup", "delivery", "service", "replacement"].into_iter().collect();

    let all_jobs = ctx.problem.plan.jobs.iter().map(|job| (job.id.clone(), job.clone())).collect::<HashMap<_, _>>();
    let mut used_jobs = HashMap::<String, JobAssignment>::new();

    ctx.solution.tours.iter().try_for_each(|tour| {
        tour.stops
            .iter()
            .flat_map(|stop| stop.activities.iter())
            .enumerate()
            .filter(|(_, activity)| activity_types.contains(&activity.activity_type.as_str()))
            .try_for_each(|(idx, activity)| {
                let tour_info = (tour.vehicle_id.clone(), tour.shift_index);
                let asgn =
                    used_jobs.entry(activity.job_id.clone()).or_insert_with(|| new_assignment(tour_info.clone()));

                if asgn.tour_info != tour_info {
                    return Err(format!("Job served in multiple tours: '{}'", activity.job_id));
                }

                match activity.activity_type.as_str() {
                    "pickup" => asgn.pickups.push(idx),
                    "delivery" => asgn.deliveries.push(idx),
                    "service" => asgn.services.push(idx),
                    "replacement" => asgn.replacements.push(idx),
                    _ => {}
                }

                Ok(())
            })
    })?;

    used_jobs.iter().try_for_each(|(id, asgn)| {
        // TODO validate whether each job task is served once
        let job = all_jobs.get(id).ok_or_else(|| format!("Cannot find job with id {}", id))?;
        let expected_tasks = job.pickups.as_ref().map_or(0, |p| p.len())
            + job.deliveries.as_ref().map_or(0, |d| d.len())
            + job.services.as_ref().map_or(0, |s| s.len())
            + job.replacements.as_ref().map_or(0, |r| r.len());
        let assigned_tasks = asgn.pickups.len() + asgn.deliveries.len() + asgn.services.len() + asgn.replacements.len();

        if expected_tasks != assigned_tasks {
            return Err(format!(
                "Not all tasks served for '{}', expected: {}, assigned: {}",
                id, expected_tasks, assigned_tasks
            ));
        }

        if !asgn.deliveries.is_empty() && asgn.pickups.iter().max() > asgn.deliveries.iter().min() {
            return Err(format!("Found pickup after delivery for '{}'", id));
        }

        Ok(())
    })?;

    let all_unassigned_jobs = ctx
        .solution
        .unassigned
        .iter()
        .flat_map(|jobs| jobs.iter().filter(|job| !job.job_id.ends_with("_break")))
        .map(|job| job.job_id.clone())
        .collect::<Vec<_>>();

    let unique_unassigned_jobs = all_unassigned_jobs.iter().cloned().collect::<HashSet<_>>();

    if unique_unassigned_jobs.len() != all_unassigned_jobs.len() {
        return Err("Duplicated job ids in the list of unassigned jobs".to_string());
    }

    unique_unassigned_jobs.iter().try_for_each(|job_id| {
        if !all_jobs.contains_key(job_id) {
            return Err(format!("Unknown job id in the list of unassigned jobs: '{}'", job_id));
        }

        if used_jobs.contains_key(job_id) {
            return Err(format!("Job present as assigned and unassigned: '{}'", job_id));
        }

        Ok(())
    })?;

    let all_used_job =
        unique_unassigned_jobs.into_iter().chain(used_jobs.into_iter().map(|(id, _)| id)).collect::<Vec<_>>();

    if all_used_job.len() != all_jobs.len() {
        return Err(format!(
            "Amount of jobs present in problem and solution doesn't match: {} vs {}",
            all_jobs.len(),
            all_used_job.len()
        ));
    }

    Ok(())
}

/// Checks job constraint violations.
fn check_jobs_match(ctx: &CheckerContext) -> Result<(), String> {
    let job_ids = ctx
        .solution
        .tours
        .iter()
        .flat_map(move |tour| {
            tour.stops.iter().flat_map(move |stop| {
                stop.activities
                    .iter()
                    .filter(move |activity| {
                        try_match_job(
                            tour,
                            stop,
                            activity,
                            get_job_index(&ctx.core_problem),
                            get_coord_index(&ctx.core_problem),
                        )
                        .is_err()
                    })
                    .map(|activity| {
                        format!(
                            "{}:{}",
                            activity.job_id.clone(),
                            activity.job_tag.as_ref().unwrap_or(&"<no tag>".to_string())
                        )
                    })
            })
        })
        .collect::<Vec<_>>();

    if !job_ids.is_empty() {
        return Err(format!("cannot match activities to jobs: {}", job_ids.join(", ")));
    }

    Ok(())
}
