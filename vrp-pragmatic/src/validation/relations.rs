#[cfg(test)]
#[path = "../../tests/unit/validation/relations_test.rs"]
mod relations_test;

use super::*;
use std::collections::HashMap;

/// Checks that relation job ids are defined in plan.
fn check_e1200_job_existence(relations: &Vec<Relation>, job_map: &HashMap<String, &Job>) -> Result<(), FormatError> {
    let job_ids = relations
        .iter()
        .flat_map(|relation| {
            relation
                .jobs
                .iter()
                .filter(|&job_id| {
                    job_id != "departure" && job_id != "arrival" && job_id != "break" && job_id != "reload"
                })
                .filter(|&job_id| !job_map.contains_key(job_id))
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

/// Validates relations in the plan.
pub fn validate_relations(ctx: &ValidationContext) -> Result<(), Vec<FormatError>> {
    let job_map = ctx.jobs().map(|job| (job.id.clone(), job)).collect::<HashMap<_, _>>();
    let vehicle_map = ctx
        .vehicles()
        .map(|v_type| v_type)
        .flat_map(|v_type| v_type.vehicle_ids.iter().map(move |id| (id.clone(), v_type)))
        .collect::<HashMap<_, _>>();

    if let Some(relations) = ctx.problem.plan.relations.as_ref() {
        combine_error_results(&[
            check_e1200_job_existence(relations, &job_map),
            check_e1201_vehicle_existence(relations, &vehicle_map),
            check_e1202_empty_job_list(relations),
        ])
    } else {
        Ok(())
    }
}
