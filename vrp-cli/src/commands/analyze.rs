use super::*;
use vrp_cli::extensions::analyze::get_clusters;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const MIN_POINTS_ARG_NAME: &str = "jobs-size";
const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_analyze_app<'a, 'b>() -> App<'a, 'b> {
    App::new("analyze").about("Provides helper functionality to analyze problem or solution").subcommand(
        App::new("clusters")
            .about("Analyzes job clusters")
            .arg(
                Arg::with_name(FORMAT_ARG_NAME)
                    .help("Specifies input type")
                    .required(true)
                    .possible_values(&["pragmatic"])
                    .index(1),
            )
            .arg(Arg::with_name(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
            .arg(
                Arg::with_name(MIN_POINTS_ARG_NAME)
                    .help("Minimum cluster size")
                    .short("c")
                    .default_value("4")
                    .long(MIN_POINTS_ARG_NAME)
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name(MATRIX_ARG_NAME)
                    .help("Specifies path to file with routing matrix")
                    .short("m")
                    .long(MATRIX_ARG_NAME)
                    .multiple(true)
                    .required(false)
                    .takes_value(true),
            )
            .arg(
                Arg::with_name(OUT_RESULT_ARG_NAME)
                    .help("Specifies path to the file for result output")
                    .short("o")
                    .long(OUT_RESULT_ARG_NAME)
                    .required(false)
                    .takes_value(true),
            ),
    )
}

pub fn run_analyze(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> Result<(), String> {
    match matches.subcommand() {
        ("clusters", Some(clusters_matches)) => {
            let problem_path = matches.value_of(PROBLEM_ARG_NAME).unwrap();
            let problem_format = matches.value_of(FORMAT_ARG_NAME).unwrap();

            if problem_format != "pragmatic" {
                return Err(format!("unknown problem format: '{}'", problem_format));
            }

            let problem_reader = BufReader::new(open_file(problem_path, "problem"));

            let matrices_readers = clusters_matches
                .values_of(MATRIX_ARG_NAME)
                .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

            let min_points = parse_int_value::<usize>(matches, MIN_POINTS_ARG_NAME, "min points")?;
            let clusters = get_clusters(problem_reader, matrices_readers, min_points)
                .map_err(|err| format!("cannot get clusters: '{}'", err))?;

            let out_geojson = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out geojson"));
            let mut geo_writer = out_writer_func(out_geojson);

            geo_writer.write_all(clusters.as_bytes()).map_err(|err| format!("cannot write result: '{}'", err))
        }
        ("", None) => {
            return Err(format!("no subcommand was used. Use -h to print help information."));
        }
        _ => unreachable!(),
    }
}
