#[cfg(test)]
#[path = "../../tests/unit/checker/relations_test.rs"]
mod relations_test;

use super::*;
use crate::utils::combine_error_results;
use hashbrown::HashSet;

/// Checks relation rules.
pub fn check_relations(context: &CheckerContext) -> Result<(), Vec<GenericError>> {
    combine_error_results(&[check_relations_assignment(context)])
}

fn check_relations_assignment(context: &CheckerContext) -> Result<(), GenericError> {
    let reserved_ids = vec!["departure", "arrival", "break", "dispatch", "reload"].into_iter().collect::<HashSet<_>>();

    (0_usize..)
        .zip(context.problem.plan.relations.as_ref().map_or([].iter(), |relations| relations.iter()))
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
                    Err(format!("relation has unknown job id: {job_id}"))
                }
            })?;

            if expected_relation_count != relation.jobs.len() {
                return Err(format!("relation {} contains duplicated ids: {:?}", idx, relation.jobs).into());
            }

            match relation.type_field {
                RelationType::Strict => {
                    let common = intersection(activity_ids.clone(), relation.jobs.clone());
                    if common != relation.jobs {
                        Err(format!(
                            "relation {} does not follow strict rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, common
                        )
                        .into())
                    } else {
                        Ok(())
                    }
                }
                RelationType::Sequence => {
                    let ids = activity_ids.iter().filter(|id| relation_ids.contains(id)).cloned().collect::<Vec<_>>();
                    if ids != relation.jobs {
                        Err(format!(
                            "relation {} does not follow sequence rule: expected {:?}, got {:?}, common: {:?}",
                            idx, relation.jobs, activity_ids, ids
                        )
                        .into())
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
                        Err(format!("relation {idx} has jobs assigned to another tour").into())
                    } else {
                        Ok(())
                    }
                }
            }
        })?;

    Ok(())
}

fn get_tour_by_vehicle_id(
    vehicle_id: &str,
    shift_index: Option<usize>,
    solution: &Solution,
) -> Result<Tour, GenericError> {
    solution
        .tours
        .iter()
        .find(|tour| tour.vehicle_id == vehicle_id && tour.shift_index == shift_index.unwrap_or(0))
        .cloned()
        .ok_or_else(|| format!("cannot find tour for '{vehicle_id}'").into())
}

fn get_activity_ids(tour: &Tour) -> Vec<String> {
    tour.stops
        .iter()
        .flat_map(|stop| {
            // TODO consider job tags within multi jobs
            stop.activities().iter().map(|a| a.job_id.clone())
        })
        .collect()
}

fn intersection<T>(left: Vec<T>, right: Vec<T>) -> Vec<T>
where
    T: PartialEq,
{
    if right.is_empty() {
        return vec![];
    }

    if let Some(position) = left.iter().position(|item| *item == *right.first().unwrap()) {
        left.into_iter().skip(position).zip(right).filter(|(a, b)| *a == *b).map(|(item, _)| item).collect()
    } else {
        vec![]
    }
}
