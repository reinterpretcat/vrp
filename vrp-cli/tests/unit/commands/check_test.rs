use crate::cli::{get_app, run_subcommand};

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";
const PRAGMATIC_MATRIX_PATH: &str = "../examples/data/pragmatic/simple.basic.matrix.json";
const PRAGMATIC_SOLUTION_PATH: &str = "../examples/data/pragmatic/simple.basic.solution.json";

#[test]
fn can_run_check_solution() {
    let args = vec![
        "vrp-cli",
        "check",
        "pragmatic",
        "--problem-file",
        PRAGMATIC_PROBLEM_PATH,
        "--matrix",
        PRAGMATIC_MATRIX_PATH,
        "--solution-file",
        PRAGMATIC_SOLUTION_PATH,
    ];
    let matches = get_app().try_get_matches_from(args).unwrap();

    run_subcommand(matches);
}
