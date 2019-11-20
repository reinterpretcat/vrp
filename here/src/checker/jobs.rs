use crate::checker::models::*;
use std::collections::HashSet;

pub fn check_jobs(solution: &SolutionInfo) -> Result<(), String> {
    check_job_presence(solution)?;

    solution.tours.iter().try_for_each::<_, Result<_, String>>(|tour| {
        check_stop_has_proper_demand_change(tour)?;
        check_activity_has_no_time_window_violation(tour)?;

        check_single_job_pd_has_all_activities_in_proper_order(tour)?;
        check_multi_job_has_all_activities_in_allowed_order(tour)?;

        Ok(())
    })?;

    Ok(())
}

fn check_job_presence(solution: &SolutionInfo) -> Result<(), String> {
    let mut assigned_job_ids = solution.tours.iter().try_fold::<_, _, Result<_, String>>(
        HashSet::<String>::default(),
        |mut out_acc, tour| {
            let others = tour.activities.iter().try_fold(HashSet::<String>::default(), |mut in_acc, act| {
                if let Some(job_id) = act.job_id.as_ref() {
                    if out_acc.get(job_id).is_some() {
                        return Err(format!(
                            "Job '{}' used second time in another tour: '{}'",
                            job_id, tour.vehicle_meta.vehicle_id
                        ));
                    }
                    in_acc.insert(job_id.clone());
                }

                Ok(in_acc)
            })?;

            out_acc.extend(others.into_iter());

            Ok(out_acc)
        },
    )?;

    assigned_job_ids.extend(solution.unassigned.iter().map(|job| job.unassigned.job_id.clone()));

    if assigned_job_ids.len() < solution.jobs.len() {
        return Err(format!(
            "Solution has less jobs than the problem: {} < {}",
            assigned_job_ids.len(),
            solution.jobs.len()
        ));
    }

    if assigned_job_ids.len() > solution.jobs.len() {
        return Err(format!(
            "Solution has more jobs than the problem: {} > {}",
            assigned_job_ids.len(),
            solution.jobs.len()
        ));
    }

    Ok(())
}

fn check_stop_has_proper_demand_change(tour: &TourInfo) -> Result<(), String> {
    unimplemented!()
}

fn check_activity_has_no_time_window_violation(tour: &TourInfo) -> Result<(), String> {
    unimplemented!()
}

fn check_single_job_pd_has_all_activities_in_proper_order(tour: &TourInfo) -> Result<(), String> {
    unimplemented!()
}

fn check_multi_job_has_all_activities_in_allowed_order(tour: &TourInfo) -> Result<(), String> {
    unimplemented!()
}
