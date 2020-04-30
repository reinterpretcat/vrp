use crate::extensions::generate::generate_problem;
use std::fs::File;
use std::io::BufReader;

#[test]
fn can_generate_problem_from_simple_prototype() {
    let reader = BufReader::new(File::open("../examples/data/pragmatic/simple.basic.problem.json").unwrap());
    let problem = generate_problem("pragmatic", Some(vec![reader]), 50).map_err(|err| panic!(err)).unwrap();

    // TODO add more checks
    assert_eq!(problem.plan.jobs.len(), 50);
}
