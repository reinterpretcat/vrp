use super::*;
use std::fs::File;
use std::io::BufReader;
use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;

#[test]
pub fn can_get_clusters() {
    let problem_reader = BufReader::new(
        File::open("../examples/data/pragmatic/benches/simple.deliveries.100.json").expect("cannot read problem file"),
    );

    let problem = deserialize_problem(problem_reader).unwrap().read_pragmatic().unwrap();
    let locations = get_dbscan_clusters(&problem, None, None).expect("cannot get cluster");
    let clusters = serialize_named_locations_as_geojson(&locations).unwrap();

    assert!(clusters.contains("features"));
    assert!(clusters.contains("geometry"));
    assert!(clusters.contains("Point"));
}
