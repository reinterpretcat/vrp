use super::*;
use crate::json::problem::*;
use std::ops::Range;
use std::rc::Rc;
use std::sync::RwLock;

/// Generate relations.
pub fn generate_relations(
    jobs: &Vec<JobVariant>,
    vehicles: &Vec<VehicleType>,
    total_relation_amount: Range<usize>,
    jobs_per_relation: Range<usize>,
) -> impl Strategy<Value = Vec<Relation>> {
    let job_ids = Rc::new(RwLock::new(get_job_ids(jobs)));
    let vehicle_ids = get_vehicle_ids(vehicles);

    // NOTE this is done to reduce rejections by proptest
    let max = total_relation_amount.end.min(jobs.len() / jobs_per_relation.end).max(1);
    let min = max.min(total_relation_amount.start).max(0);

    prop::collection::vec(
        generate_relation(job_ids.clone(), vehicle_ids.clone(), jobs_per_relation.clone())
            .prop_filter("Relation with no job ids", move |relation| relation.jobs.len() > jobs_per_relation.start),
        min..max,
    )
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
    prop_oneof![Just(RelationType::Sequence), Just(RelationType::Flexible), Just(RelationType::Tour)]
}

fn get_job_ids(jobs: &Vec<JobVariant>) -> Vec<String> {
    jobs.iter()
        .map(|job| match job {
            JobVariant::Single(job) => job.id.clone(),
            _ => todo!("Multi job in relation generator is not yet supported."),
        })
        .collect()
}

fn get_vehicle_ids(vehicles: &Vec<VehicleType>) -> Vec<String> {
    vehicles.iter().map(|vehicle| vehicle.id.clone()).collect()
}
