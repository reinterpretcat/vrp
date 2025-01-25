use super::*;
use crate::cli::{get_app, run_subcommand};

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";

#[test]
fn can_run_analyze_dbscan() {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let args = vec![
        "vrp-cli",
        "analyze",
        "dbscan",
        "pragmatic",
        PRAGMATIC_PROBLEM_PATH,
        "--out-result",
        tmpfile.path().to_str().unwrap(),
    ];
    let matches = get_app().try_get_matches_from(args).unwrap();

    run_subcommand(matches);
}

#[test]
fn can_detect_wrong_argument_in_dbscan() {
    let args = vec!["analyze", "dbscan", "solomon", PRAGMATIC_PROBLEM_PATH, "--out-result", "/some/path"];

    assert!(get_analyze_app().try_get_matches_from(args).is_err());
}

#[test]
fn can_run_analyze_kmedoids() {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let args = vec![
        "vrp-cli",
        "analyze",
        "kmedoids",
        "pragmatic",
        PRAGMATIC_PROBLEM_PATH,
        "-k",
        "3",
        "--out-result",
        tmpfile.path().to_str().unwrap(),
    ];
    let matches = get_app().try_get_matches_from(args).unwrap();

    run_subcommand(matches);
}
