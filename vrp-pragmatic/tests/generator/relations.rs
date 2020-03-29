use super::*;
use crate::json::problem::*;
use std::ops::Range;
use std::rc::Rc;
use std::sync::RwLock;

/// Generate relations.
pub fn generate_relations(
    jobs: &Vec<Job>,
    vehicles: &Vec<VehicleType>,
    total_relations: Range<usize>,
    jobs_per_relation: Range<usize>,
) -> impl Strategy<Value = Vec<Relation>> {
    let job_ids = Rc::new(RwLock::new(get_job_ids(jobs)));
    let vehicle_ids = get_vehicle_ids(vehicles);

    // NOTE this is done to reduce rejections by proptest
    let max = total_relations.end.min(jobs.len() / jobs_per_relation.end).max(1);
    let min = max.min(total_relations.start).max(0);
    let max = if min == max { max + 1 } else { max };

    prop::collection::vec(generate_relation(job_ids.clone(), vehicle_ids.clone(), jobs_per_relation.clone()), min..max)
        .prop_filter_map(
            "Empty relations in plan",
            |relations| {
                if relations.is_empty() {
                    None
                } else {
                    Some(relations)
                }
            },
        )
}

fn generate_relation(
    job_ids: Rc<RwLock<Vec<String>>>,
    vehicles: Vec<String>,
    jobs_per_relation: Range<usize>,
) -> impl Strategy<Value = Relation> {
    let vehicle_count = vehicles.len();

    get_relation_type()
        .prop_flat_map(move |relation_type| (Just(relation_type), 0..vehicle_count))
        .prop_flat_map(move |(relation_type, vehicle_idx)| {
            let vehicle_id = vehicles.get(vehicle_idx).cloned().unwrap();
            (Just(relation_type), Just(vehicle_id), jobs_per_relation.clone())
        })
        .prop_map(move |(relation_type, vehicle_id, job_count)| {
            let len = job_count.min(job_ids.read().unwrap().len());
            let jobs = if job_count > 0 { job_ids.write().unwrap().drain(0..len).collect::<Vec<_>>() } else { vec![] };

            Relation { type_field: relation_type, jobs, vehicle_id, shift_index: None }
        })
        // NOTE prop_filter behaves in strange way
        .prop_filter_map(
            "Empty jobs in relation",
            |relation| {
                if relation.jobs.is_empty() {
                    None
                } else {
                    Some(relation)
                }
            },
        )
}

fn get_relation_type() -> impl Strategy<Value = RelationType> {
    prop_oneof![Just(RelationType::Strict), Just(RelationType::Sequence), Just(RelationType::Any)]
}

fn get_job_ids(jobs: &Vec<Job>) -> Vec<String> {
    jobs.iter().map(|job| job.id.clone()).collect()
}

fn get_vehicle_ids(vehicles: &Vec<VehicleType>) -> Vec<String> {
    vehicles.iter().flat_map(|vehicle| vehicle.vehicle_ids.iter().cloned()).collect()
}
