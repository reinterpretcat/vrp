#[cfg(test)]
#[path = "../../tests/unit/checker/skills_test.rs"]
mod skills_test;

use super::*;
use vrp_core::prelude::GenericResult;

/// Checks that every served job's skill requirement is met by the vehicle serving it.
///
/// The rules mirror `vrp_core::construction::features::skills`, which is the constraint the
/// solver enforces during insertion: `allOf` must be a subset of the vehicle's skills, `oneOf`
/// needs at least one match, and `noneOf` must be disjoint. A job demanding skills a vehicle
/// does not hold is infeasible however good the route looks.
pub fn check_skills(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    let violations = context
        .solution
        .tours
        .iter()
        .map(|tour| {
            let vehicle = context.get_vehicle(&tour.vehicle_id)?;
            let vehicle_skills = vehicle.skills.as_ref().map(|skills| skills.iter().cloned().collect::<HashSet<_>>());

            let violations = tour
                .stops
                .iter()
                .flat_map(|stop| stop.activities().iter())
                .filter_map(|activity| context.get_job_by_id(&activity.job_id))
                .filter_map(|job| job.skills.as_ref().map(|skills| (job, skills)))
                .filter(|(_, skills)| !is_satisfied(skills, vehicle_skills.as_ref()))
                .map(|(job, _)| {
                    format!(
                        "job '{}' requires skills its vehicle does not hold, vehicle id '{}', shift index: {}",
                        job.id, tour.vehicle_id, tour.shift_index
                    )
                })
                .collect::<Vec<_>>();

            Ok(violations)
        })
        .collect::<GenericResult<Vec<_>>>()
        .map_err(|err| vec![err])?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if violations.is_empty() {
        Ok(())
    } else {
        // Keep one error per distinct message: a job repeated across activities (a multi job
        // visited twice) would otherwise report the same violation more than once.
        let mut seen = HashSet::new();
        Err(violations.into_iter().filter(|msg| seen.insert(msg.clone())).map(GenericError::from).collect())
    }
}

fn is_satisfied(job_skills: &JobSkills, vehicle_skills: Option<&HashSet<String>>) -> bool {
    check_all_of(demand(job_skills.all_of.as_ref()), vehicle_skills)
        && check_one_of(demand(job_skills.one_of.as_ref()), vehicle_skills)
        && check_none_of(demand(job_skills.none_of.as_ref()), vehicle_skills)
}

/// An empty list is not a demand of zero skills, it is the absence of a demand — the reading
/// `JobSkills::new` bakes in by mapping an empty vector to `None` before the constraint ever sees
/// it. Our emitter writes `"oneOf": []` on every unrestricted job, so taking the list at face value
/// would report the whole plan as unskilled.
fn demand(skills: Option<&Vec<String>>) -> Option<&Vec<String>> {
    skills.filter(|skills| !skills.is_empty())
}

fn check_all_of(demanded: Option<&Vec<String>>, held: Option<&HashSet<String>>) -> bool {
    match (demanded, held) {
        (Some(demanded), Some(held)) => demanded.iter().all(|skill| held.contains(skill)),
        (Some(_), None) => false,
        _ => true,
    }
}

fn check_one_of(demanded: Option<&Vec<String>>, held: Option<&HashSet<String>>) -> bool {
    match (demanded, held) {
        (Some(demanded), Some(held)) => demanded.iter().any(|skill| held.contains(skill)),
        (Some(_), None) => false,
        _ => true,
    }
}

fn check_none_of(demanded: Option<&Vec<String>>, held: Option<&HashSet<String>>) -> bool {
    match (demanded, held) {
        (Some(demanded), Some(held)) => !demanded.iter().any(|skill| held.contains(skill)),
        _ => true,
    }
}
