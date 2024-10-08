use crate::helpers::{create_lc101_problem, get_job_ids, get_vehicle_capacity};

#[test]
fn can_read_lilim_format_from_test_file() {
    let problem = create_lc101_problem();

    assert_eq!(get_job_ids(&problem), (0..53).map(|i| i.to_string()).collect::<Vec<String>>());
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    assert_eq!(get_vehicle_capacity(&problem), 200);
}
