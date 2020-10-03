use super::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::process;
use std::sync::Arc;
use vrp_cli::extensions::check::check_pragmatic_solution;
use vrp_cli::extensions::solve::config::create_builder_from_config_file;
use vrp_cli::{get_errors_serialized, get_locations_serialized};
use vrp_core::models::{Problem, Solution};
use vrp_core::solver::{Builder, Metrics, Telemetry, TelemetryMode};

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const GENERATIONS_ARG_NAME: &str = "max-generations";
const TIME_ARG_NAME: &str = "max-time";
const COST_VARIATION_ARG_NAME: &str = "cost-variation";
const GEO_JSON_ARG_NAME: &str = "geo-json";

const INIT_SOLUTION_ARG_NAME: &str = "init-solution";
const OUT_RESULT_ARG_NAME: &str = "out-result";
const GET_LOCATIONS_ARG_NAME: &str = "get-locations";
const CONFIG_ARG_NAME: &str = "config";
const LOG_ARG_NAME: &str = "log";
const CHECK_ARG_NAME: &str = "check";
const RANDOM_SEED_NAME: &str = "seed";

#[allow(clippy::type_complexity)]
struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Result<Solution, String>>);

#[allow(clippy::type_complexity)]
struct SolutionWriter(
    pub  Box<
        dyn Fn(
            &Problem,
            Solution,
            Option<Metrics>,
            BufWriter<Box<dyn Write>>,
            Option<BufWriter<Box<dyn Write>>>,
        ) -> Result<(), String>,
    >,
);

#[allow(clippy::type_complexity)]
struct LocationWriter(pub Box<dyn Fn(File, BufWriter<Box<dyn Write>>) -> Result<(), String>>);

#[allow(clippy::type_complexity)]
type FormatMap<'a> = HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter, LocationWriter)>;

fn add_scientific(formats: &mut FormatMap) {
    if cfg!(feature = "scientific-format") {
        use vrp_scientific::lilim::{LilimProblem, LilimSolution};
        use vrp_scientific::solomon::read_init_solution as read_init_solomon;
        use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};

        formats.insert(
            "solomon",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_solomon()
                })),
                InitSolutionReader(Box::new(|file, problem| read_init_solomon(BufReader::new(file), problem))),
                SolutionWriter(Box::new(|_, solution, _, writer, _| solution.write_solomon(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
        formats.insert(
            "lilim",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_lilim()
                })),
                InitSolutionReader(Box::new(|_file, _problem| unimplemented!())),
                SolutionWriter(Box::new(|_, solution, _, writer, _| solution.write_lilim(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
    }
}

fn add_pragmatic(formats: &mut FormatMap) {
    use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem};
    use vrp_pragmatic::format::solution::read_init_solution as read_init_pragmatic;
    use vrp_pragmatic::format::solution::PragmaticSolution;

    formats.insert(
        "pragmatic",
        (
            ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                if let Some(matrices) = matrices {
                    let matrices = matrices.into_iter().map(BufReader::new).collect();
                    (BufReader::new(problem), matrices).read_pragmatic()
                } else {
                    BufReader::new(problem).read_pragmatic()
                }
                .map_err(|errors| errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join("\t\n"))
            })),
            InitSolutionReader(Box::new(|file, problem| read_init_pragmatic(BufReader::new(file), problem))),
            SolutionWriter(Box::new(|problem, solution, metrics, default_writer, geojson_writer| {
                geojson_writer
                    .map_or(Ok(()), |geojson_writer| solution.write_geo_json(problem, geojson_writer))
                    .and_then(|_| {
                        if let Some(metrics) = metrics {
                            (solution, metrics).write_pragmatic_json(problem, default_writer)
                        } else {
                            solution.write_pragmatic_json(problem, default_writer)
                        }
                    })
            })),
            LocationWriter(Box::new(|problem, writer| {
                let mut writer = writer;
                deserialize_problem(BufReader::new(problem))
                    .map_err(|errors| get_errors_serialized(&errors))
                    .and_then(|problem| get_locations_serialized(&problem))
                    .and_then(|locations| writer.write_all(locations.as_bytes()).map_err(|err| err.to_string()))
            })),
        ),
    );
}

fn get_formats<'a>() -> FormatMap<'a> {
    let mut formats = FormatMap::default();

    add_scientific(&mut formats);
    add_pragmatic(&mut formats);

    formats
}

pub fn get_solve_app<'a, 'b>() -> App<'a, 'b> {
    App::new("solve")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies the problem type")
                .required(true)
                .possible_values(&["solomon", "lilim", "pragmatic"])
                .index(1),
        )
        .arg(Arg::with_name(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
        .arg(
            Arg::with_name(GENERATIONS_ARG_NAME)
                .help("Specifies maximum number of generations")
                .short("n")
                .long(GENERATIONS_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(TIME_ARG_NAME)
                .help("Specifies max time algorithm run in seconds")
                .short("t")
                .long(TIME_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(COST_VARIATION_ARG_NAME)
                .help("Specifies cost variation coefficient termination criteria in form \"sample_size,threshold\"")
                .short("v")
                .long(COST_VARIATION_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(INIT_SOLUTION_ARG_NAME)
                .help("Specifies path to file with initial solution")
                .short("i")
                .long(INIT_SOLUTION_ARG_NAME)
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
                .help("Specifies path to file for result output")
                .short("o")
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(GET_LOCATIONS_ARG_NAME)
                .help("Returns list of unique locations")
                .short("l")
                .long(GET_LOCATIONS_ARG_NAME)
                .required(false),
        )
        .arg(
            Arg::with_name(GEO_JSON_ARG_NAME)
                .help("Specifies path to solution output in geo json format")
                .short("g")
                .long(GEO_JSON_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(CONFIG_ARG_NAME)
                .help("Specifies path to algorithm configuration file")
                .short("c")
                .long(CONFIG_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(LOG_ARG_NAME)
                .help("Specifies whether default logging is enabled")
                .long(LOG_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(CHECK_ARG_NAME)
                .help("Specifies whether final solution should be checked for feasibility")
                .long(CHECK_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name(RANDOM_SEED_NAME)
                .help("Specifies randomization seed to avoid stochastic behavior")
                .long(RANDOM_SEED_NAME)
                .required(false)
                .takes_value(true),
        )
}

/// Runs solver commands.
pub fn run_solve(matches: &ArgMatches) {
    let formats = get_formats();

    // required
    let problem_path = matches.value_of(PROBLEM_ARG_NAME).unwrap();
    let problem_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let problem_file = open_file(problem_path, "problem");

    // optional
    let max_generations = parse_int_value::<usize>(matches, GENERATIONS_ARG_NAME, "max generations");
    let max_time = parse_int_value::<usize>(matches, TIME_ARG_NAME, "max time");
    let telemetry = Telemetry::new(if matches.is_present(LOG_ARG_NAME) {
        TelemetryMode::OnlyLogging { logger: Arc::new(|msg| println!("{}", msg)), log_best: 100, log_population: 1000 }
    } else {
        TelemetryMode::None
    });
    let is_check_requested = matches.is_present(CHECK_ARG_NAME);

    let cost_variation = matches.value_of(COST_VARIATION_ARG_NAME).map(|arg| {
        if let [sample, threshold] =
            arg.split(',').filter_map(|line| line.parse::<f64>().ok()).collect::<Vec<_>>().as_slice()
        {
            (*sample as usize, *threshold)
        } else {
            eprintln!("cannot parse cost variation");
            process::exit(1);
        }
    });
    let init_solution = matches.value_of(INIT_SOLUTION_ARG_NAME).map(|path| open_file(path, "init solution"));
    let config = matches.value_of(CONFIG_ARG_NAME).map(|path| open_file(path, "config"));
    let matrix_files = get_matrix_files(matches);
    let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out solution"));
    let out_geojson = matches.value_of(GEO_JSON_ARG_NAME).map(|path| create_file(path, "out geojson"));
    let is_get_locations_set = matches.is_present(GET_LOCATIONS_ARG_NAME);
    let seed = parse_int_value::<u64>(matches, RANDOM_SEED_NAME, "seed");

    match formats.get(problem_format) {
        Some((problem_reader, init_reader, solution_writer, locations_writer)) => {
            let out_buffer = create_write_buffer(out_result);
            let geo_buffer = out_geojson.map(|geojson| create_write_buffer(Some(geojson)));

            if is_get_locations_set {
                locations_writer.0(problem_file, out_buffer).unwrap_or_else(|err| {
                    eprintln!("cannot get locations '{}'", err);
                    process::exit(1);
                });
            } else {
                match problem_reader.0(problem_file, matrix_files) {
                    Ok(problem) => {
                        let problem = Arc::new(problem);
                        let solutions = init_solution.map_or_else(Vec::new, |file| {
                            init_reader.0(file, problem.clone())
                                .map_err(|err| {
                                    eprintln!("cannot read initial solution '{}'", err);
                                    process::exit(1);
                                })
                                .map(|solution| vec![solution])
                                .unwrap()
                        });

                        let builder = if let Some(config) = config {
                            create_builder_from_config_file(problem.clone(), BufReader::new(config)).unwrap_or_else(
                                |err| {
                                    eprintln!("cannot read config: '{}'", err);
                                    process::exit(1);
                                },
                            )
                        } else {
                            Builder::new(problem.clone())
                                .with_max_generations(max_generations)
                                .with_max_time(max_time)
                                .with_cost_variation(cost_variation)
                                .with_telemetry(telemetry)
                                .with_seed(seed)
                        };

                        let (solution, _, metrics) = builder
                            .with_init_solutions(solutions)
                            .build()
                            .and_then(|solver| solver.solve())
                            .unwrap_or_else(|err| {
                                eprintln!("cannot find any solution: '{}'", err);
                                process::exit(1);
                            });

                        solution_writer.0(&problem, solution, metrics, out_buffer, geo_buffer).unwrap();

                        if is_check_requested {
                            check_solution(matches);
                        }
                    }
                    Err(error) => {
                        eprintln!("cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                        process::exit(1);
                    }
                };
            }
        }
        None => {
            eprintln!("unknown format: '{}'", problem_format);
            process::exit(1);
        }
    }
}

fn get_matrix_files(matches: &ArgMatches) -> Option<Vec<File>> {
    matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| open_file(path, "routing matrix")).collect())
}

fn check_solution(matches: &ArgMatches) {
    let problem_file = matches
        .value_of(PROBLEM_ARG_NAME)
        .map(|path| BufReader::new(open_file(path, "problem")))
        .expect("cannot read problem");
    let solution_file = matches
        .value_of(OUT_RESULT_ARG_NAME)
        .map(|path| BufReader::new(open_file(path, "solution")))
        .expect("cannot read solution");

    let matrix_files = matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

    let result = check_pragmatic_solution(problem_file, solution_file, matrix_files);

    if let Err(err) = result {
        eprintln!("{}", err);
        process::exit(1);
    } else {
        println!("solution feasibility check is completed successfully");
    }
}
