use crate::extensions::solve::config::{create_builder_from_config, read_config};
use crate::pragmatic::format::problem::PragmaticProblem;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

#[test]
fn can_solve_problem_using_full_config() {
    let problem = Arc::new(
        BufReader::new(File::open("../examples/data/pragmatic/simple.basic.problem.json").unwrap())
            .read_pragmatic()
            .unwrap(),
    );
    let reader = BufReader::new(File::open("../examples/data/config/config.full.json").unwrap());
    // TODO override termination to avoid test timeout on CI
    let mut config = read_config(reader).unwrap();
    if let Some(termination) = config.termination.as_mut() {
        termination.max_time = Some(10);
    }

    let (solution, _, _) = create_builder_from_config(problem, &config).unwrap().build().unwrap().solve().unwrap();

    assert!(!solution.routes.is_empty())
}
