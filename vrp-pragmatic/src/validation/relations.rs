#[cfg(test)]
#[path = "../../tests/unit/validation/relations_test.rs"]
mod relations_test;

use super::*;
use crate::utils::combine_error_results;
use hashbrown::HashSet;
use vrp_core::utils::CollectGroupBy;

/// Checks that relation job ids are defined in plan.
fn check_e1200_job_existence(ctx: &ValidationContext, relations: &[Relation]) -> Result<(), FormatError> {
    let job_ids = relations
        .iter()
        .flat_map(|relation| {
            relation
                .jobs
                .iter()
                .filter(|&job_id| !is_reserved_job_id(job_id))
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
    relations: &[Relation],
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
fn check_e1202_empty_job_list(relations: &[Relation]) -> Result<(), FormatError> {
    let has_empty_relations = relations.iter().any(|relation| !relation.jobs.iter().any(|id| !is_reserved_job_id(id)));

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
fn check_e1203_no_multiple_places_times(ctx: &ValidationContext, relations: &[Relation]) -> Result<(), FormatError> {
    let mut job_ids = relations
        .iter()
        .flat_map(|relation| {
            relation
                .jobs
                .iter()
                .filter(|&job_id| !is_reserved_job_id(job_id))
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

/// Checks that relation job is assigned to one vehicle.
fn check_e1204_job_assigned_to_multiple_vehicles(relations: &[Relation]) -> Result<(), FormatError> {
    let mut job_vehicle_map = HashMap::<String, String>::new();
    let job_ids: Vec<String> = relations
        .iter()
        .flat_map(|relation| {
            relation
                .jobs
                .clone()
                .into_iter()
                .filter(|job_id| !is_reserved_job_id(job_id))
                .filter(|job_id| {
                    *job_vehicle_map.entry(job_id.clone()).or_insert_with(|| relation.vehicle_id.clone())
                        != relation.vehicle_id
                })
                .collect::<Vec<String>>()
                .into_iter()
        })
        .collect::<Vec<_>>();

    if job_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1204".to_string(),
            "job is assigned to different vehicles in relations".to_string(),
            format!("assign jobs only to one vehicle, ids: '{}'", job_ids.join(", ")),
        ))
    }
}

fn check_e1205_relation_has_correct_shift_index(
    relations: &[Relation],
    vehicle_map: &HashMap<String, &VehicleType>,
) -> Result<(), FormatError> {
    let vehicle_ids: Vec<String> = relations
        .iter()
        .filter_map(|relation| vehicle_map.get(&relation.vehicle_id).map(|vehicle| (vehicle, relation)))
        .filter(|(vehicle, relation)| vehicle.shifts.get(relation.shift_index.unwrap_or(0)).is_none())
        .map(|(_, relation)| relation.vehicle_id.clone())
        .collect::<Vec<_>>();

    if vehicle_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1205".to_string(),
            "relation has invalid shift index".to_string(),
            format!(
                "check that vehicle has enough shifts defined or correct relation, vehicle ids: '{}'",
                vehicle_ids.join(", ")
            ),
        ))
    }
}

/// Checks that relation has no reserved job ids for vehicle shift properties which are not used.
fn check_e1206_relation_has_no_missing_shift_properties(
    relations: &[Relation],
    vehicle_map: &HashMap<String, &VehicleType>,
) -> Result<(), FormatError> {
    let vehicle_ids: Vec<String> = relations
        .iter()
        .filter_map(|relation| {
            vehicle_map
                .get(&relation.vehicle_id)
                .and_then(|vehicle| vehicle.shifts.get(relation.shift_index.unwrap_or(0)))
                .map(|vehicle_shift| (vehicle_shift, relation))
        })
        .filter(|(vehicle_shift, relation)| {
            relation.jobs.iter().filter(|job_id| is_reserved_job_id(job_id)).any(|job_id| match job_id.as_str() {
                "break" => vehicle_shift.breaks.is_none(),
                "dispatch" => vehicle_shift.dispatch.is_none(),
                "reload" => vehicle_shift.reloads.is_none(),
                "arrival" => vehicle_shift.end.is_none(),
                _ => false,
            })
        })
        .map(|(_, relation)| relation.vehicle_id.clone())
        .collect::<Vec<_>>();

    if vehicle_ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1206".to_string(),
            "relation has special job id which is not defined on vehicle shift".to_string(),
            format!(
                "remove special job id or add vehicle shift property \
            (e.g. break, dispatch, reload), vehicle ids: '{}'",
                vehicle_ids.join(", ")
            ),
        ))
    }
}

fn check_e1207_no_incomplete_relation(ctx: &ValidationContext, relations: &[Relation]) -> Result<(), FormatError> {
    let get_tasks_size = |tasks: &Option<Vec<JobTask>>| {
        if let Some(tasks) = tasks {
            tasks.len()
        } else {
            0
        }
    };

    let ids = relations
        .iter()
        .filter_map(|relation| {
            let job_frequencies = relation.jobs.iter().collect_group_by_key(|&job| job);
            let ids = relation
                .jobs
                .iter()
                .filter_map(|job_id| ctx.job_index.get(job_id))
                .filter(|job| {
                    let size = get_tasks_size(&job.pickups)
                        + get_tasks_size(&job.deliveries)
                        + get_tasks_size(&job.replacements)
                        + get_tasks_size(&job.services);

                    job_frequencies.get(&job.id).unwrap().len() != size
                })
                .map(|job| job.id.clone())
                .collect::<Vec<_>>();

            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    if ids.is_empty() {
        Ok(())
    } else {
        Err(FormatError::new(
            "E1207".to_string(),
            "some relations have incomplete job definitions".to_string(),
            format!(
                "ensure that job id specified in relation as many times, as it has tasks, problematic job ids: '{}'",
                ids.join(", ")
            ),
        ))
    }
}

/// Validates relations in the plan.
pub fn validate_relations(ctx: &ValidationContext) -> Result<(), MultiFormatError> {
    let vehicle_map = ctx
        .vehicles()
        .flat_map(|v_type| v_type.vehicle_ids.iter().map(move |id| (id.clone(), v_type)))
        .collect::<HashMap<_, _>>();

    if let Some(relations) = ctx.problem.plan.relations.as_ref() {
        combine_error_results(&[
            check_e1200_job_existence(ctx, relations),
            check_e1201_vehicle_existence(relations, &vehicle_map),
            check_e1202_empty_job_list(relations),
            check_e1203_no_multiple_places_times(ctx, relations),
            check_e1204_job_assigned_to_multiple_vehicles(relations),
            check_e1205_relation_has_correct_shift_index(relations, &vehicle_map),
            check_e1206_relation_has_no_missing_shift_properties(relations, &vehicle_map),
            check_e1207_no_incomplete_relation(ctx, relations),
        ])
        .map_err(|errors| errors.into())
    } else {
        Ok(())
    }
}
