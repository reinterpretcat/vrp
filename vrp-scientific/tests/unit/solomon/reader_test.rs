use crate::helpers::*;
use crate::solomon::SolomonProblem;

#[test]
fn can_read_solomon_built_from_builder() {
    let problem = SolomonBuilder::default()
        .set_title("Three customers")
        .set_vehicle((2, 10))
        .add_customer((0, 0, 0, 0, 0, 1000, 1))
        .add_customer((1, 1, 0, 1, 5, 1000, 5))
        .add_customer((2, 3, 0, 2, 0, 1002, 11))
        .add_customer((3, 7, 0, 1, 0, 1000, 12))
        .build()
        .read_solomon(false)
        .unwrap();

    assert_eq!(get_job_ids(&problem), vec!["1", "2", "3"]);
    assert_eq!(get_job_demands(&problem), vec![1, 2, 1]);
    assert_eq!(get_vehicle_capacity(&problem), 10);
    assert_eq!(get_job_time_windows(&problem), vec![(5., 1000.), (0., 1002.), (0., 1000.)]);
    assert_eq!(get_job_durations(&problem), vec![5., 11., 12.]);

    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 2);
}

#[test]
fn can_read_solomon_format_from_test_file() {
    let problem = create_c101_25_problem();

    assert_eq!(get_job_ids(&problem), (1..26).map(|i| i.to_string()).collect::<Vec<String>>());
    assert_eq!(
        get_job_demands(&problem),
        vec![10, 30, 10, 10, 10, 20, 20, 20, 10, 10, 10, 20, 30, 10, 40, 40, 20, 20, 10, 10, 20, 20, 10, 10, 40]
    );
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    assert_eq!(get_vehicle_capacity(&problem), 200);
}
