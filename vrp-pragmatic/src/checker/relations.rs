#[cfg(test)]
#[path = "../../tests/unit/checker/relations_test.rs"]
mod relations_test;

use super::*;
use std::collections::HashSet;

/// Checks relation rules.
pub fn check_relations(context: &CheckerContext) -> Result<(), String> {
    let reserved_ids = vec!["departure", "arrival", "break", "reload"].into_iter().collect::<HashSet<_>>();

    (0_usize..)
        .zip(context.problem.plan.relations.as_ref().map_or(vec![].iter(), |relations| relations.iter()))
        .try_for_each(|(idx, relation)| {
            let tour = get_tour_by_vehicle_id(&relation.vehicle_id, relation.shift_index, &context.solution);
            // NOTE tour can be absent for tour relation
            let tour = if let Ok(tour) = tour {
                tour
            } else {
                return match relation.type_field {
                    RelationType::Any => Ok(()),
                    _ => tour.map(|_| ()),
                };
            };

            let activity_ids = get_activity_ids(&tour);
            let relation_ids = relation.jobs.iter().collect::<HashSet<_>>();

            let expected_relation_count = relation_ids.iter().try_fold(0, |acc, job_id| {
                if let Some(job) = context.get_job_by_id(job_id) {
                    Ok(acc
                        + job.pickups.as_ref().map_or(0, |t| t.len())
                        + job.deliveries.as_ref().map_or(0, |t| t.len())
                        + job.replacements.as_ref().map_or(0, |t| t.len())
                        + job.services.as_ref().map_or(0, |t| t.len()))
                } else if reserved_ids.contains(job_id.as_str()) {
                    Ok(acc + 1)
                } else {
                    Err(format!("Relation has unknown job id: {}", job_id))
                }
            })?;

            if expected_relation_count != relation.jobs.len() {
                return Err(format!("Relation {} contains duplicated ids: {:?}", idx, relation.jobs));
            }

            match relation.type_field {
                RelationType::Strict => {
                    let common = intersection(activity_ids.clone(), relation.jobs.clone());
                    if common != relation.jobs {
                        Err(format!(
                            "Relation {} does not follow strict rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, common
                        ))
                    } else {
                        Ok(())
                    }
                }
                RelationType::Sequence => {
                    let ids = activity_ids.iter().filter(|id| relation_ids.contains(id)).cloned().collect::<Vec<_>>();
                    if ids != relation.jobs {
                        Err(format!(
                            "Relation {} does not follow sequence rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, ids
                        ))
                    } else {
                        Ok(())
                    }
                }
                RelationType::Any => {
                    let has_wrong_assignment = context
                        .solution
                        .tours
                        .iter()
                        .filter(|other| tour.vehicle_id != other.vehicle_id)
                        .any(|tour| get_activity_ids(tour).iter().any(|id| relation_ids.contains(id)));

                    if has_wrong_assignment {
                        Err(format!("Relation {} has jobs assigned to another tour", idx))
                    } else {
                        Ok(())
                    }
                }
            }
        })?;

    Ok(())
}

fn get_tour_by_vehicle_id(vehicle_id: &str, shift_index: Option<usize>, solution: &Solution) -> Result<Tour, String> {
    solution
        .tours
        .iter()
        .find(|tour| tour.vehicle_id == vehicle_id && tour.shift_index == shift_index.unwrap_or(0))
        .cloned()
        .ok_or_else(|| format!("Cannot find tour for '{}'", vehicle_id))
}

fn get_activity_ids(tour: &Tour) -> Vec<String> {
    tour.stops
        .iter()
        .flat_map(|stop| {
            // TODO consider job tags within multi jobs
            stop.activities.iter().map(|a| a.job_id.clone())
        })
        .collect()
}

fn intersection<T>(left: Vec<T>, right: Vec<T>) -> Vec<T>
where
    T: PartialEq,
{
    let mut common = Vec::new();
    let mut right = right;

    for e1 in left.into_iter() {
        if let Some(pos) = right.iter().position(|e2| e1 == *e2) {
            common.push(e1);
            right.remove(pos);
        } else if !common.is_empty() {
            break;
        }
    }

    common
}
