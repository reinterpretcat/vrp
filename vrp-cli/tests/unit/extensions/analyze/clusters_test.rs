use super::*;
use std::fs::File;
use std::io::BufReader;
use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;

#[test]
pub fn can_get_dbscan_clusters() {
    can_get_clusters(|problem| get_dbscan_clusters(problem, None, None));
}

#[test]
pub fn can_get_kmedoids_clusters() {
    can_get_clusters(|problem| get_k_medoids_clusters(problem, 2));
}

type LocationResult = GenericResult<Vec<(String, ApiLocation, usize)>>;

fn can_get_clusters(clusters_fn: fn(&Problem) -> LocationResult) {
    let problem_reader = BufReader::new(
        File::open("../examples/data/pragmatic/benches/simple.deliveries.100.json").expect("cannot read problem file"),
    );

    let problem = deserialize_problem(problem_reader).unwrap().read_pragmatic().unwrap();
    let locations = clusters_fn(&problem).unwrap();
    let clusters = serialize_named_locations_as_geojson(&locations).unwrap();

    assert!(clusters.contains("features"));
    assert!(clusters.contains("geometry"));
    assert!(clusters.contains("Point"));
}
