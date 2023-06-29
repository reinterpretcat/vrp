use crate::extensions::solve::config::{create_builder_from_config, read_config};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_core::prelude::Solver;
use vrp_pragmatic::format::problem::PragmaticProblem;

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
    if let Some(initial) = config.evolution.as_mut().and_then(|evolution| evolution.initial.as_mut()) {
        initial.alternatives.max_size = 1;
    }
    if let Some(termination) = config.termination.as_mut() {
        termination.max_generations = Some(1);
        termination.max_time = None;
        termination.variation = None;
    }

    let (solution, _, _) = create_builder_from_config(problem.clone(), Default::default(), &config)
        .unwrap()
        .build()
        .map(|config| Solver::new(problem.clone(), config))
        .unwrap()
        .solve()
        .unwrap();

    assert!(!solution.routes.is_empty())
}
