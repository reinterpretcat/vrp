use crate::construction::constraints::CapacityDimension;
use crate::construction::constraints::Demand;
use crate::helpers::get_test_resource;
use crate::helpers::models::problem::{get_job_id, get_job_simple_demand};
use crate::streams::input::text::solomon::parse_solomon_format;
use std::fs::File;
use std::io::BufReader;

#[test]
fn can_read_solomon_format_from_test_file() {
    let file = get_test_resource("data/solomon/C101.25.txt").unwrap();
    let mut reader = BufReader::new(file);

    let problem = parse_solomon_format(reader).unwrap();

    assert_eq!(
        problem.jobs.all().map(|j| get_job_id(j.as_ref()).to_owned()).collect::<Vec<String>>(),
        (1..26).map(|i| ["c".to_string(), i.to_string()].concat()).collect::<Vec<String>>()
    );
    assert_eq!(
        problem.jobs.all().map(|j| get_job_simple_demand(j.as_ref()).delivery.0).collect::<Vec<i32>>(),
        vec![10, 30, 10, 10, 10, 20, 20, 20, 10, 10, 10, 20, 30, 10, 40, 40, 20, 20, 10, 10, 20, 20, 10, 10, 40]
    );
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 25);
    let capacity: i32 = *problem.fleet.vehicles.iter().next().unwrap().dimens.get_capacity().unwrap();
    assert_eq!(capacity, 200);
}
