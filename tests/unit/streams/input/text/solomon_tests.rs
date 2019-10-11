use crate::construction::constraints::CapacityDimension;
use crate::construction::constraints::Demand;
use crate::helpers::get_test_resource;
use crate::helpers::models::problem::{get_job_id, get_job_simple_demand};
use crate::helpers::streams::input::SolomonBuilder;
use crate::models::common::TimeWindow;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::streams::input::text::solomon::SolomonProblem;

fn get_job_ids(problem: &Problem) -> Vec<String> {
    problem.jobs.all().map(|j| get_job_id(j.as_ref()).to_owned()).collect()
}

fn get_job_demands(problem: &Problem) -> Vec<i32> {
    problem.jobs.all().map(|j| get_job_simple_demand(j.as_ref()).delivery.0).collect()
}

fn get_vehicle_capacity(problem: &Problem) -> i32 {
    *problem.fleet.vehicles.iter().next().unwrap().dimens.get_capacity().unwrap()
}

fn get_job_time_windows(problem: &Problem) -> Vec<(f64, f64)> {
    problem
        .jobs
        .all()
        .map(|j| match j.as_ref() {
            Job::Single(j) => j.places.first().unwrap().times.first().map(|tw| (tw.start, tw.end)).unwrap(),
            _ => panic!(),
        })
        .collect()
}

fn get_job_durations(problem: &Problem) -> Vec<f64> {
    problem
        .jobs
        .all()
        .map(|j| match j.as_ref() {
            Job::Single(j) => j.places.first().unwrap().duration,
            _ => panic!(),
        })
        .collect()
}

#[test]
fn can_read_solomon_built_from_builder() {
    let problem = SolomonBuilder::new()
        .set_title("Three customers")
        .set_vehicle((2, 10))
        .add_customer((0, 0, 0, 0, 0, 1000, 1))
        .add_customer((1, 1, 0, 1, 5, 1000, 5))
        .add_customer((2, 3, 0, 2, 0, 1002, 11))
        .add_customer((3, 7, 0, 1, 0, 1000, 12))
        .build()
        .parse_solomon()
        .unwrap();

    assert_eq!(get_job_ids(&problem), vec!["c1", "c2", "c3"]);
    assert_eq!(get_job_demands(&problem), vec![1, 2, 1]);
    assert_eq!(get_vehicle_capacity(&problem), 10);
    assert_eq!(get_job_time_windows(&problem), vec![(5., 1000.), (0., 1002.), (0., 1000.)]);
    assert_eq!(get_job_durations(&problem), vec![5., 11., 12.]);

    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 2);
}

#[test]
fn can_read_solomon_format_from_test_file() {
    let problem = get_test_resource("data/solomon/C101.25.txt").unwrap().parse_solomon().unwrap();

    assert_eq!(
        get_job_ids(&problem),
        (1..26).map(|i| ["c".to_string(), i.to_string()].concat()).collect::<Vec<String>>()
    );
    assert_eq!(
        get_job_demands(&problem),
        vec![10, 30, 10, 10, 10, 20, 20, 20, 10, 10, 10, 20, 30, 10, 40, 40, 20, 20, 10, 10, 20, 20, 10, 10, 40]
    );
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    assert_eq!(get_vehicle_capacity(&problem), 200);
}
