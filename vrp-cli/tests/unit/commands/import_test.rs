use super::*;
use crate::cli::{get_app, run_subcommand};
use vrp_cli::pragmatic::format::problem::PragmaticProblem;

const CSV_JOBS_PATH: &str = "../examples/data/csv/jobs.csv";
const VEHICLES_JOBS_PATH: &str = "../examples/data/csv/vehicles.csv";

#[test]
fn can_import_csv_problem_from_args() {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let args = vec![
        "vrp-cli",
        "import",
        "csv",
        "--input-files",
        CSV_JOBS_PATH,
        VEHICLES_JOBS_PATH,
        "--out-result",
        tmpfile.path().to_str().unwrap(),
    ];
    let matches = get_app().try_get_matches_from(args).unwrap();

    run_subcommand(matches);

    let problem = BufReader::new(tmpfile.as_file()).read_pragmatic().unwrap();
    assert_eq!(problem.jobs.size(), 3);
    assert_eq!(problem.fleet.vehicles.len(), 30);
}
