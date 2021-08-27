use super::*;
use std::fs::File;

#[test]
pub fn can_get_clusters() {
    let problem = BufReader::new(
        File::open("../examples/data/pragmatic/benches/simple.deliveries.100.json").expect("cannot read problem file"),
    );

    let clusters = get_clusters(problem, None, None).expect("cannot get cluster");

    assert!(clusters.contains("features"));
    assert!(clusters.contains("geometry"));
    assert!(clusters.contains("Point"));
}
