#[cfg(test)]
#[path = "../../tests/unit/commands/analyze_test.rs"]
mod analyze_test;

use super::*;
use vrp_cli::extensions::analyze::get_clusters;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const MIN_POINTS_ARG_NAME: &str = "min-points";
const EPSILON_ARG_NAME: &str = "epsilon";
const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_analyze_app() -> Command<'static> {
    Command::new("analyze").about("Provides helper functionality to analyze problem or solution").subcommand(
        Command::new("clusters")
            .about("Analyzes job clusters")
            .arg(
                Arg::new(FORMAT_ARG_NAME)
                    .help("Specifies input type")
                    .required(true)
                    .possible_values(&["pragmatic"])
                    .index(1),
            )
            .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
            .arg(
                Arg::new(MIN_POINTS_ARG_NAME)
                    .help("Minimum cluster size")
                    .short('c')
                    .default_value("3")
                    .long(MIN_POINTS_ARG_NAME)
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::new(EPSILON_ARG_NAME)
                    .help("Epsilon parameter in DBSCAN")
                    .short('e')
                    .long(EPSILON_ARG_NAME)
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::new(MATRIX_ARG_NAME)
                    .help("Specifies path to file with routing matrix")
                    .short('m')
                    .long(MATRIX_ARG_NAME)
                    .multiple_values(true)
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::new(OUT_RESULT_ARG_NAME)
                    .help("Specifies path to the file for result output")
                    .short('o')
                    .long(OUT_RESULT_ARG_NAME)
                    .required(true)
                    .takes_value(true),
            ),
    )
}

pub fn run_analyze(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> Result<(), String> {
    match matches.subcommand() {
        Some(("clusters", clusters_matches)) => {
            let problem_path = clusters_matches.value_of(PROBLEM_ARG_NAME).unwrap();
            let problem_format = clusters_matches.value_of(FORMAT_ARG_NAME).unwrap();

            if problem_format != "pragmatic" {
                return Err(format!("unknown problem format: '{}'", problem_format));
            }

            let problem_reader = BufReader::new(open_file(problem_path, "problem"));

            let matrices_readers = clusters_matches
                .values_of(MATRIX_ARG_NAME)
                .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

            let min_points = parse_int_value::<usize>(clusters_matches, MIN_POINTS_ARG_NAME, "min points")?;
            let epsilon = parse_float_value::<f64>(clusters_matches, EPSILON_ARG_NAME, "epsilon")?;

            let clusters = get_clusters(problem_reader, matrices_readers, min_points, epsilon)
                .map_err(|err| format!("cannot get clusters: '{}'", err))?;

            let out_geojson =
                clusters_matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out geojson"));
            let mut geo_writer = out_writer_func(out_geojson);

            geo_writer.write_all(clusters.as_bytes()).map_err(|err| format!("cannot write result: '{}'", err))
        }
        _ => Err("no argument with analyze subcommand was used. Use -h to print help information".to_string()),
    }
}
