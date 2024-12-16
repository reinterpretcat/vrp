#[cfg(test)]
#[path = "../../tests/unit/commands/analyze_test.rs"]
mod analyze_test;

use super::*;
use std::sync::Arc;
use vrp_cli::extensions::analyze::{get_dbscan_clusters, get_k_medoids_clusters};
use vrp_core::prelude::*;
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;
use vrp_pragmatic::format::Location as ApiLocation;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const MIN_POINTS_ARG_NAME: &str = "min-points";
const EPSILON_ARG_NAME: &str = "epsilon";
const K_ARG_NAME: &str = "k";
const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_analyze_app() -> Command {
    Command::new("analyze")
        .about("Provides helper functionality to analyze problem or solution")
        .subcommand(
            Command::new("dbscan")
                .about("Analyzes job clusters using dbscan algorithm")
                .arg(
                    Arg::new(FORMAT_ARG_NAME)
                        .help("Specifies input type")
                        .required(true)
                        .value_parser(["pragmatic"])
                        .index(1),
                )
                .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
                .arg(
                    Arg::new(MIN_POINTS_ARG_NAME)
                        .help("Minimum cluster size")
                        .short('c')
                        .default_value("3")
                        .long(MIN_POINTS_ARG_NAME)
                        .required(false),
                )
                .arg(
                    Arg::new(EPSILON_ARG_NAME)
                        .help("Epsilon parameter in DBSCAN")
                        .short('e')
                        .long(EPSILON_ARG_NAME)
                        .required(false),
                )
                .arg(
                    Arg::new(MATRIX_ARG_NAME)
                        .help("Specifies path to file with routing matrix")
                        .short('m')
                        .long(MATRIX_ARG_NAME)
                        .num_args(1..)
                        .required(false),
                )
                .arg(
                    Arg::new(OUT_RESULT_ARG_NAME)
                        .help("Specifies path to the file for result output")
                        .short('o')
                        .long(OUT_RESULT_ARG_NAME)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("kmedoids")
                .about("Analyzes job clusters using kmedoids algorithm")
                .arg(
                    Arg::new(FORMAT_ARG_NAME)
                        .help("Specifies input type")
                        .required(true)
                        .value_parser(["pragmatic"])
                        .index(1),
                )
                .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
                .arg(
                    Arg::new(K_ARG_NAME)
                        .help("Number of clusters (k) to create")
                        .short('k')
                        .default_value("2")
                        .required(false),
                )
                .arg(
                    Arg::new(MATRIX_ARG_NAME)
                        .help("Specifies path to file with routing matrix")
                        .short('m')
                        .long(MATRIX_ARG_NAME)
                        .num_args(1..)
                        .required(false),
                )
                .arg(
                    Arg::new(OUT_RESULT_ARG_NAME)
                        .help("Specifies path to the file for result output")
                        .short('o')
                        .long(OUT_RESULT_ARG_NAME)
                        .required(true),
                ),
        )
}

pub fn run_analyze(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> GenericResult<()> {
    match matches.subcommand() {
        Some(("dbscan", clusters_matches)) => {
            let min_points = parse_int_value::<usize>(clusters_matches, MIN_POINTS_ARG_NAME, "min points")?;
            let epsilon = parse_float_value::<Float>(clusters_matches, EPSILON_ARG_NAME, "epsilon")?;

            read_and_execute_clusters_command(clusters_matches, out_writer_func, |problem| {
                get_dbscan_clusters(problem, min_points, epsilon)
            })
        }
        Some(("kmedoids", clusters_matches)) => {
            let k = parse_int_value::<usize>(clusters_matches, K_ARG_NAME, "k")?;

            read_and_execute_clusters_command(clusters_matches, out_writer_func, |problem| {
                get_k_medoids_clusters(problem, k.unwrap_or(2))
            })
        }
        _ => Err("no argument with analyze subcommand was used. Use -h to print help information".into()),
    }
}

fn read_and_execute_clusters_command<F>(
    clusters_matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
    command_fn: F,
) -> GenericResult<()>
where
    F: Fn(&Problem) -> GenericResult<Vec<(String, ApiLocation, usize)>>,
{
    let problem_path = clusters_matches.get_one::<String>(PROBLEM_ARG_NAME).unwrap();
    let problem_format = clusters_matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();
    if problem_format != "pragmatic" {
        return Err(format!("unknown problem format: '{problem_format}'").into());
    }

    let problem_reader = BufReader::new(open_file(problem_path, "problem"));
    let matrices_readers = clusters_matches
        .get_many::<String>(MATRIX_ARG_NAME)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

    let problem = Arc::new(get_core_problem(problem_reader, matrices_readers).map_err(|errs| errs.to_string())?);
    let locations = command_fn(&problem)?;
    let result = serialize_named_locations_as_geojson(locations.as_slice())?;

    let out_geojson =
        clusters_matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out geojson"));
    let mut geo_writer = out_writer_func(out_geojson);

    geo_writer.write_all(result.as_bytes()).map_err(|err| format!("cannot write result: '{err}'").into())
}
