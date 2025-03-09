use crate::helpers::get_test_resource;
use crate::tsplib::TsplibProblem;
use crate::tsplib::reader::TsplibReader;
use std::fs::File;
use std::io::{BufReader, Read};

fn get_example_problem_string() -> String {
    let mut buffer = "".to_string();

    get_test_resource("../../examples/data/scientific/tsplib/example.txt")
        .expect("cannot open file")
        .read_to_string(&mut buffer)
        .expect("cannot read file");

    buffer
}

fn get_example_problem_reader() -> BufReader<File> {
    BufReader::new(get_test_resource("../../examples/data/scientific/tsplib/example.txt").expect("cannot open file"))
}

#[test]
fn can_read_meta_errors() {
    for &(from, to, error) in &[
        ("CVRP", "ASD", "expecting 'CVRP' as TYPE, got 'ASD'"),
        ("DIMENSION : 6", "DIMENSION : asd", "cannot parse DIMENSION: 'invalid float literal'"),
        ("EUC_2D", "ASD", "expecting 'EUC_2D' as EDGE_WEIGHT_TYPE, got 'ASD'"),
        ("CAPACITY : 30", "CAPACITY : asd", "cannot parse CAPACITY: 'invalid float literal'"),
    ] {
        let content = get_example_problem_string().replace(from, to);
        let mut reader = TsplibReader::new(BufReader::new(content.as_bytes()));

        let result = reader.read_meta();

        assert_eq!(result, Err(error.into()));
    }
}

#[test]
fn can_read_meta_capacity_and_dimension() {
    let mut reader = TsplibReader::new(get_example_problem_reader());

    reader.read_meta().expect("cannot read meta");

    assert_eq!(reader.dimension, Some(6));
    assert_eq!(reader.vehicle_capacity, Some(30));
}

#[test]
fn can_read_customer_data() {
    let mut reader = TsplibReader::new(get_example_problem_reader());
    reader.read_meta().expect("cannot read meta");

    let (coordinates, demands) = reader.read_customer_data().expect("cannot read coordinates");

    assert_eq!(coordinates.len(), 6);
    assert_eq!(demands.len(), 6);
}

#[test]
fn can_read_depot_data() {
    let mut reader = TsplibReader::new(get_example_problem_reader());
    reader.read_meta().expect("cannot read meta");
    reader.read_customer_data().expect("cannot read customer data");

    assert_eq!(reader.read_depot_data(), Ok(1));
}

#[test]
fn can_read_problem() {
    let reader = get_example_problem_reader();

    let problem = reader.read_tsplib(false).expect("cannot read problem");

    assert_eq!(problem.jobs.size(), 5);
    assert_eq!(problem.fleet.actors.len(), 6);
}
