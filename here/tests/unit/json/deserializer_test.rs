use crate::helpers::get_test_resource;
use crate::json::deserializer::deserialize_problem;
use std::io::BufReader;

#[test]
fn can_deserialize_problem() {
    let file = get_test_resource("../data/small/minimal.problem.json").unwrap();

    let problem = deserialize_problem(BufReader::new(file)).unwrap();

    assert_eq!(problem.id, "Minimal problem with 2 jobs, 1 vehicle, 4 locations");
}
