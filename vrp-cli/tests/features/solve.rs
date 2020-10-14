use crate::extensions::solve::config::create_builder_from_config_file;
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

    let (solution, _, _) = create_builder_from_config_file(problem, reader).unwrap().build().unwrap().solve().unwrap();

    assert!(!solution.routes.is_empty())
}
