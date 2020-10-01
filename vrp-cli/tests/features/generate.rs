use crate::extensions::generate::generate_problem;
use std::fs::File;
use std::io::BufReader;
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::validation::ValidationContext;

#[test]
fn can_generate_problem_from_simple_prototype() {
    let reader = BufReader::new(File::open("../examples/data/pragmatic/simple.basic.problem.json").unwrap());
    let problem = generate_problem("pragmatic", Some(vec![reader]), 50, 4, None).map_err(|err| panic!(err)).unwrap();

    ValidationContext::new(&problem, None)
        .validate()
        .map_err(|err| panic!(FormatError::format_many(&err, "\t\n")))
        .unwrap();

    // TODO add more checks
    assert_eq!(problem.plan.jobs.len(), 50);
    assert_eq!(problem.fleet.vehicles.len(), 4);
}
