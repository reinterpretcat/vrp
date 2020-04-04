#[cfg(test)]
#[path = "../../tests/unit/validation/relations_test.rs"]
mod relations_test;

use super::*;
use std::collections::{HashMap, HashSet};

/// Checks that relation job ids are defined in plan.
fn check_e1200_job_existence(ctx: &ValidationContext, relations: &Vec<Relation>) -> Result<(), FormatError> {
    let job_ids = relations
        .iter()
        .flat_map(|relation| {
            relation
                .jobs
                .iter()
                .filter(|&job_id| filter_non_jobs(job_id))
                .filter(|&job_id| !ctx.job_index.contains_key(job_id))
                .cloned()
        })
        .collect::<Vec<_>>();

    if job_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1200".to_string(),
            "relation has job id which does not present in the plan".to_string(),
            format!("remove from relations or add jobs to the plan, ids: '{}'", job_ids.join(", ")),
        ))
    }
}

/// Checks that relation vehicle ids are defined in fleet.
fn check_e1201_vehicle_existence(
    relations: &Vec<Relation>,
    vehicle_map: &HashMap<String, &VehicleType>,
) -> Result<(), FormatError> {
    let vehicle_ids = relations
        .iter()
        .map(|relation| relation.vehicle_id.clone())
        .filter(|vehicle_id| !vehicle_map.contains_key(vehicle_id))
        .collect::<Vec<_>>();

    if vehicle_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1201".to_string(),
            "relation has vehicle id which does not present in the fleet".to_string(),
            format!("remove from relations or add vehicle types to the fleet, ids: '{}'", vehicle_ids.join(", ")),
        ))
    }
}

/// Checks that relation vehicle ids are defined in fleet.
fn check_e1202_empty_job_list(relations: &Vec<Relation>) -> Result<(), FormatError> {
    let has_empty_relations = relations.iter().any(|relation| relation.jobs.is_empty());

    if has_empty_relations {
        Err(FormatError::new(
            "E1202".to_string(),
            "relation has empty job id list".to_string(),
            "remove relation with empty jobs list or add job ids to them".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that relation has no jobs with multiple places or time windows.
fn check_e1203_no_multiple_places_times(ctx: &ValidationContext, relations: &Vec<Relation>) -> Result<(), FormatError> {
    let mut job_ids = relations
        .iter()
        .filter(|relation| match relation.type_field {
            RelationType::Any => false,
            _ => true,
        })
        .flat_map(|relation| {
            relation
                .jobs
                .iter()
                .filter(|&job_id| filter_non_jobs(job_id))
                .filter_map(|job_id| ctx.job_index.get(job_id))
                .filter(|&job| {
                    ctx.tasks(job).into_iter().any(|task| {
                        task.places.len() > 1
                            || task.places.iter().any(|place| place.times.as_ref().map_or(false, |tw| tw.len() > 1))
                    })
                })
                .map(|job| job.id.clone())
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    job_ids.sort();

    if job_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1203".to_string(),
            "strict or sequence relation has job with multiple places or time windows".to_string(),
            format!(
                "remove job from relation or specify only one place and time window, job ids: '{}'",
                job_ids.join(", ")
            ),
        ))
    }
}

fn filter_non_jobs(job_id: &String) -> bool {
    job_id != "departure" && job_id != "arrival" && job_id != "break" && job_id != "reload"
}

/// Validates relations in the plan.
pub fn validate_relations(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    let vehicle_map = ctx
        .vehicles()
        .map(|v_type| v_type)
        .flat_map(|v_type| v_type.vehicle_ids.iter().map(move |id| (id.clone(), v_type)))
        .collect::<HashMap<_, _>>();

    if let Some(relations) = ctx.problem.plan.relations.as_ref() {
        combine_error_results(&[
            check_e1200_job_existence(ctx, relations),
            check_e1201_vehicle_existence(relations, &vehicle_map),
            check_e1202_empty_job_list(relations),
            check_e1203_no_multiple_places_times(ctx, relations),
        ])
    } else {
        Ok(())
    }
}
