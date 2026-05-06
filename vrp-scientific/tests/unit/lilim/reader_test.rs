use crate::helpers::{LilimBuilder, create_lc101_problem, get_job_ids, get_vehicle_capacity};
use crate::lilim::LilimProblem;
use vrp_core::construction::features::JobDemandDimension;
use vrp_core::models::common::{Demand, SingleDimLoad};
use vrp_core::models::problem::Job;

#[test]
fn can_read_lilim_format_from_test_file() {
    let problem = create_lc101_problem();

    assert_eq!(get_job_ids(&problem), (0..53).map(|i| i.to_string()).collect::<Vec<String>>());
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    assert_eq!(get_vehicle_capacity(&problem), 200);
}

#[test]
fn can_read_lilim_format_properly() {
    let problem = LilimBuilder::new()
        .set_vehicle((3, 15))
        .add_customer((0, 0, 0, 0, 0, 1000, 0, 0, 0)) // depot
        .add_customer((1, 1, 0, 15, 0, 1000, 5, 0, 2)) // pickup, delivers to 2
        .add_customer((2, 2, 0, -15, 0, 1000, 5, 1, 0)) // delivery, picked from 1
        .add_customer((3, 3, 0, 10, 0, 1000, 3, 0, 4)) // pickup, delivers to 4
        .add_customer((4, 4, 0, -10, 0, 1000, 3, 3, 0)) // delivery, picked from 3
        .build()
        .read_lilim(false)
        .unwrap();

    assert_eq!(problem.fleet.vehicles.len(), 3);
    assert_eq!(get_vehicle_capacity(&problem), 15);
    assert_eq!(get_job_ids(&problem), vec!["0", "1"]);

    // verify demands on sub-jobs of each multi job
    let jobs: Vec<_> = problem.jobs.all().into_iter().collect();

    let (pickup_demand_0, delivery_demand_0) = get_multi_job_demands(&jobs[0]);
    assert_eq!(pickup_demand_0.pickup.1.value, 15);
    assert_eq!(pickup_demand_0.delivery.1.value, 0);
    assert_eq!(delivery_demand_0.delivery.1.value, 15);
    assert_eq!(delivery_demand_0.pickup.1.value, 0);

    let (pickup_demand_1, delivery_demand_1) = get_multi_job_demands(&jobs[1]);
    assert_eq!(pickup_demand_1.pickup.1.value, 10);
    assert_eq!(pickup_demand_1.delivery.1.value, 0);
    assert_eq!(delivery_demand_1.delivery.1.value, 10);
    assert_eq!(delivery_demand_1.pickup.1.value, 0);
}

fn get_multi_job_demands(job: &Job) -> (&Demand<SingleDimLoad>, &Demand<SingleDimLoad>) {
    match job {
        Job::Multi(multi) => {
            let pickup = multi.jobs[0].dimens.get_job_demand().unwrap();
            let delivery = multi.jobs[1].dimens.get_job_demand().unwrap();
            (pickup, delivery)
        }
        _ => panic!("expected multi job"),
    }
}
