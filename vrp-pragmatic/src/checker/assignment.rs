#[cfg(test)]
#[path = "../../tests/unit/checker/assignment_test.rs"]
mod assignment_test;

use super::*;
use crate::format::solution::activity_matcher::*;
use crate::format::{get_coord_index, get_job_index};
use crate::utils::combine_error_results;
use hashbrown::HashSet;
use std::cmp::Ordering;
use vrp_core::construction::clustering::vicinity::ServingPolicy;
use vrp_core::prelude::compare_floats;
use vrp_core::utils::GenericError;

/// Checks assignment of jobs and vehicles.
pub fn check_assignment(ctx: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[
        check_vehicles(ctx),
        check_jobs_presence(ctx),
        check_jobs_match(ctx),
        check_dispatch(ctx),
        check_groups(ctx),
    ])
}

/// Checks that vehicles in each tour are used once per shift and they are known in problem.
fn check_vehicles(ctx: &CheckerContext) -> Result<(), GenericError> {
    let all_vehicles: HashSet<_> = ctx.problem.fleet.vehicles.iter().flat_map(|v| v.vehicle_ids.iter()).collect();
    let mut used_vehicles = HashSet::<(String, usize)>::new();

    ctx.solution.tours.iter().try_for_each(|tour| {
        if !all_vehicles.contains(&tour.vehicle_id) {
            return Err(format!("used vehicle with unknown id: '{}'", tour.vehicle_id));
        }

        if !(used_vehicles.insert((tour.vehicle_id.to_string(), tour.shift_index))) {
            Err(format!("vehicle with '{}' id used more than once for shift {}", tour.vehicle_id, tour.shift_index))
        } else {
            Ok(())
        }
    })?;

    Ok(())
}

/// Checks job task rules.
fn check_jobs_presence(ctx: &CheckerContext) -> Result<(), GenericError> {
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
            .flat_map(|stop| stop.activities())
            .enumerate()
            .filter(|(_, activity)| activity_types.contains(&activity.activity_type.as_str()))
            .try_for_each(|(idx, activity)| {
                let tour_info = (tour.vehicle_id.clone(), tour.shift_index);
                let asgn =
                    used_jobs.entry(activity.job_id.clone()).or_insert_with(|| new_assignment(tour_info.clone()));

                if asgn.tour_info != tour_info {
                    return Err(GenericError::from(format!("job served in multiple tours: '{}'", activity.job_id)));
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
        let job = all_jobs.get(id).ok_or_else(|| format!("cannot find job with id {id}"))?;
        let expected_tasks = job.pickups.as_ref().map_or(0, |p| p.len())
            + job.deliveries.as_ref().map_or(0, |d| d.len())
            + job.services.as_ref().map_or(0, |s| s.len())
            + job.replacements.as_ref().map_or(0, |r| r.len());
        let assigned_tasks = asgn.pickups.len() + asgn.deliveries.len() + asgn.services.len() + asgn.replacements.len();

        if expected_tasks != assigned_tasks {
            return Err(GenericError::from(format!(
                "not all tasks served for '{id}', expected: {expected_tasks}, assigned: {assigned_tasks}"
            )));
        }

        if !asgn.deliveries.is_empty() && asgn.pickups.iter().max() > asgn.deliveries.iter().min() {
            return Err(GenericError::from(format!("found pickup after delivery for '{id}'")));
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
        return Err("duplicated job ids in the list of unassigned jobs".into());
    }

    unique_unassigned_jobs.iter().try_for_each::<_, Result<_, GenericError>>(|job_id| {
        if !all_jobs.contains_key(job_id) {
            return Err(format!("unknown job id in the list of unassigned jobs: '{job_id}'").into());
        }

        if used_jobs.contains_key(job_id) {
            return Err(format!("job present as assigned and unassigned: '{job_id}'").into());
        }

        Ok(())
    })?;

    let all_used_job =
        unique_unassigned_jobs.into_iter().chain(used_jobs.into_iter().map(|(id, _)| id)).collect::<Vec<_>>();

    if all_used_job.len() != all_jobs.len() {
        return Err(format!(
            "amount of jobs present in problem and solution doesn't match: {} vs {}",
            all_jobs.len(),
            all_used_job.len()
        )
        .into());
    }

    Ok(())
}

/// Checks job constraint violations.
fn check_jobs_match(ctx: &CheckerContext) -> Result<(), GenericError> {
    let job_index = get_job_index(&ctx.core_problem);
    let coord_index = get_coord_index(&ctx.core_problem);
    let job_ids = ctx
        .solution
        .tours
        .iter()
        .flat_map(move |tour| {
            tour.stops.iter().flat_map(move |stop| {
                stop.activities()
                    .iter()
                    .enumerate()
                    .filter({
                        move |(idx, activity)| {
                            match stop {
                                Stop::Point(stop) => {
                                    let result = try_match_point_job(tour, stop, activity, job_index, coord_index);
                                    match result {
                                        Err(_) => {
                                            // NOTE required break is not a job
                                            if activity.activity_type == "break" {
                                                try_match_break_activity(&ctx.problem, tour, &stop.time, activity).is_err()
                                            } else {
                                                true
                                            }
                                        },
                                        Ok(Some(JobInfo(_, _, place, time))) => {
                                            let not_equal = |left: f64, right: f64| compare_floats(left, right) != Ordering::Equal;
                                            let parking = ctx
                                                .clustering
                                                .as_ref()
                                                .map(|config| config.serving.get_parking())
                                                .unwrap_or(0.);
                                            let commute_profile = ctx.clustering.as_ref().map(|config| config.profile.clone());
                                            let domain_commute = ctx.get_commute_info(commute_profile, parking, stop, *idx);
                                            let extra_time = get_extra_time(stop, activity, &place).unwrap_or(0.);

                                            match (&ctx.clustering, &activity.commute, domain_commute) {
                                                (_, _, Err(_))
                                                | (_, None, Ok(Some(_)))
                                                | (_, Some(_), Ok(None))
                                                | (&None, &Some(_), Ok(Some(_))) => true,
                                                (_, None, Ok(None)) => {
                                                    let expected_departure = time.start.max(place.time.start) + place.duration + extra_time;
                                                    not_equal(time.end, expected_departure)
                                                }
                                                (Some(config), Some(commute), Ok(Some(d_commute))) => {
                                                    let (service_time, parking) = match config.serving {
                                                        ServingPolicy::Original { parking } => (place.duration, parking),
                                                        ServingPolicy::Multiplier { multiplier, parking } => {
                                                            (place.duration * multiplier, parking)
                                                        }
                                                        ServingPolicy::Fixed { value, parking } => (value, parking),
                                                    };

                                                    let a_commute = commute.to_domain(&ctx.coord_index);

                                                    // NOTE: we keep parking in service time of a first activity of the non-first cluster
                                                    let service_time = service_time
                                                        + if a_commute.is_zero_distance() && *idx > 0 { parking } else { 0. };

                                                    let expected_departure = time.start.max(place.time.start)
                                                        + service_time
                                                        + d_commute.backward.duration
                                                        + extra_time;
                                                    let actual_departure = time.end + d_commute.backward.duration;

                                                    // NOTE: a "workaroundish" approach for two clusters in the same stop
                                                    (not_equal(actual_departure, expected_departure)
                                                        && not_equal(actual_departure, expected_departure - parking))
                                                        // compare commute
                                                        || not_equal(a_commute.forward.distance, d_commute.forward.distance)
                                                        || not_equal(a_commute.forward.duration, d_commute.forward.duration)
                                                        || not_equal(a_commute.backward.distance, d_commute.backward.distance)
                                                        || not_equal(a_commute.backward.duration, d_commute.backward.duration)
                                                }
                                            }
                                        }
                                        _ => false,
                                    }
                                }
                                Stop::Transit(stop) => {
                                    try_match_transit_activity(&ctx.problem, tour, stop, activity).is_err()
                                }
                            }
                        }
                    })
                    .map(|(_, activity)| {
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
        return Err(format!("cannot match activities to jobs: {}", job_ids.join(", ")).into());
    }

    Ok(())
}

/// Checks whether dispatch is properly assigned.
fn check_dispatch(ctx: &CheckerContext) -> Result<(), GenericError> {
    let vehicles_with_dispatch = ctx
        .problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|v| v.shifts.iter().map(move |shift| (v.type_id.clone(), shift)))
        .filter_map(|(v, shift)| shift.dispatch.as_ref().map(|ds| (v, ds)))
        .collect::<HashMap<_, _>>();

    ctx.solution.tours.iter().try_fold::<_, _, Result<_, GenericError>>((), |_, tour| {
        let should_have_dispatch = vehicles_with_dispatch.contains_key(&tour.type_id);
        let dispatch_in_tour = tour
            .stops
            .iter()
            .enumerate()
            .flat_map(|(stop_idx, stop)| {
                stop.activities()
                    .iter()
                    .enumerate()
                    .map(move |(activity_index, activity)| (stop_idx, activity_index, activity))
            })
            .filter(|(_, _, activity)| activity.activity_type == "dispatch")
            .collect::<Vec<_>>();

        if dispatch_in_tour.len() > 1 {
            return Err(format!("more than one dispatch in the tour: '{}'", tour.vehicle_id).into());
        }

        if should_have_dispatch && dispatch_in_tour.is_empty() {
            return Err(format!("tour should have dispatch, but none is found: '{}'", tour.vehicle_id).into());
        }

        if !should_have_dispatch && !dispatch_in_tour.is_empty() {
            return Err(format!("tour should not have dispatch, but it is present: '{}'", tour.vehicle_id).into());
        }

        if should_have_dispatch {
            let (stop_idx, activity_idx, dispatch_activity) = dispatch_in_tour.first().unwrap();
            let first_stop_location = tour
                .stops
                .first()
                .unwrap()
                .as_point()
                .map(|point| point.location.clone())
                .ok_or_else(|| GenericError::from("first stop has no location"))?;

            match (stop_idx, activity_idx) {
                (0, 1) => {
                    if let Some(location) = &dispatch_activity.location {
                        if *location != first_stop_location {
                            return Err(format!(
                                "invalid dispatch location: {location}, expected to match the first stop"
                            )
                            .into());
                        }
                    }
                }
                (1, 0) => {
                    if let Some(location) = &dispatch_activity.location {
                        if *location == first_stop_location {
                            return Err(format!(
                                "invalid dispatch location: {location}, expected not to match the first stop"
                            )
                            .into());
                        }
                    }
                }
                _ => return Err(format!("invalid dispatch activity index, expected: 1, got: '{activity_idx}'").into()),
            }
        }

        Ok(())
    })
}

fn check_groups(ctx: &CheckerContext) -> Result<(), GenericError> {
    let violations = ctx
        .solution
        .tours
        .iter()
        .fold(HashMap::<String, HashSet<_>>::default(), |mut acc, tour| {
            tour.stops
                .iter()
                .flat_map(|stop| stop.activities().iter())
                .flat_map(|activity| ctx.get_job_by_id(&activity.job_id))
                .flat_map(|job| job.group.as_ref())
                .for_each(|group| {
                    acc.entry(group.clone()).or_default().insert((
                        tour.type_id.clone(),
                        tour.vehicle_id.clone(),
                        tour.shift_index,
                    ));
                });

            acc
        })
        .into_iter()
        .filter(|(_, usage)| usage.len() > 1)
        .collect::<Vec<_>>();

    if violations.is_empty() {
        Ok(())
    } else {
        let err_info = violations.into_iter().map(|(group, _)| group).collect::<Vec<_>>().join(",");
        Err(format!("job groups are not respected: '{err_info}'").into())
    }
}
