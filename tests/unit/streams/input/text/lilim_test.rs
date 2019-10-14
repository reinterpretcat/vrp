use crate::helpers::models::problem::{get_job_ids, get_vehicle_capacity};
use crate::helpers::streams::input::create_lc101_problem;

#[test]
fn can_read_lilim_format_from_test_file() {
    let problem = create_lc101_problem();

    assert_eq!(
        get_job_ids(&problem),
        (0..53).map(|i| ["mlt".to_string(), i.to_string()].concat()).collect::<Vec<String>>()
    );
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    assert_eq!(get_vehicle_capacity(&problem), 200);
}
