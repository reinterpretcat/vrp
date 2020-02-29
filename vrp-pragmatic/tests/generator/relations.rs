use super::*;
use crate::json::problem::*;
use std::ops::Range;
use std::rc::Rc;
use std::sync::RwLock;

/// Generate relations.
pub fn generate_relations(
    jobs: &Vec<Job>,
    vehicles: &Vec<VehicleType>,
    total_relation_amount: Range<usize>,
    jobs_per_relation: Range<usize>,
) -> impl Strategy<Value = Vec<Relation>> {
    let job_ids = Rc::new(RwLock::new(get_job_ids(jobs)));
    let vehicle_ids = get_vehicle_ids(vehicles);

    // NOTE this is done to reduce rejections by proptest
    let max = total_relation_amount.end.min(jobs.len() / jobs_per_relation.end).max(1);
    let min = max.min(total_relation_amount.start).max(0);
    let max = if min == max { max + 1 } else { max };

    prop::collection::vec(generate_relation(job_ids.clone(), vehicle_ids.clone(), jobs_per_relation.clone()), min..max)
}

prop_compose! {
    fn generate_relation(job_ids: Rc<RwLock<Vec<String>>>,
                         vehicles: Vec<String>,
                         jobs_per_relation: Range<usize>)
        (vehicle_idx in (0..vehicles.len()),
         job_count in jobs_per_relation,
         relation_type in get_relation_type()) -> Relation {

        let len =  job_count.min(job_ids.read().unwrap().len());
        let jobs = if job_count > 0  {
            job_ids.write().unwrap().drain(0..len).collect::<Vec<_>>()
        } else {
            vec![]
        };

        Relation {
            type_field: relation_type,
            jobs,
            vehicle_id: vehicles.get(vehicle_idx).cloned().unwrap(),
            shift_index: None
        }
    }
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
