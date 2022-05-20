#[cfg(test)]
#[path = "../../tests/unit/commands/solve_test.rs"]
mod solve_test;

use super::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vrp_cli::core::solver::TargetHeuristic;
use vrp_cli::extensions::solve::config::create_builder_from_config_file;
use vrp_cli::scientific::tsplib::{TsplibProblem, TsplibSolution};
use vrp_cli::{get_errors_serialized, get_locations_serialized};
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::problem::ProblemObjective;
use vrp_core::prelude::*;
use vrp_core::rosomaxa::evolution::*;
use vrp_core::rosomaxa::{get_default_population, get_default_selection_size};
use vrp_core::solver::*;
use vrp_core::utils::*;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const GENERATIONS_ARG_NAME: &str = "max-generations";
const TIME_ARG_NAME: &str = "max-time";
const MIN_CV_ARG_NAME: &str = "min-cv";
const GEO_JSON_ARG_NAME: &str = "geo-json";

const INIT_SOLUTION_ARG_NAME: &str = "init-solution";
const INIT_SIZE_ARG_NAME: &str = "init-size";
const OUT_RESULT_ARG_NAME: &str = "out-result";
const GET_LOCATIONS_ARG_NAME: &str = "get-locations";
const CONFIG_ARG_NAME: &str = "config";
const LOG_ARG_NAME: &str = "log";
const CHECK_ARG_NAME: &str = "check";
const SEARCH_MODE_ARG_NAME: &str = "search-mode";
const PARALLELISM_ARG_NAME: &str = "parallelism";
const HEURISTIC_ARG_NAME: &str = "heuristic";
const EXPERIMENTAL_ARG_NAME: &str = "experimental";
const ROUNDED_ARG_NAME: &str = "round";

#[allow(clippy::type_complexity)]
struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Result<Solution, String>>);

#[allow(clippy::type_complexity)]
struct SolutionWriter(
    pub  Box<
        dyn Fn(
            &Problem,
            Solution,
            f64,
            Option<TelemetryMetrics>,
            BufWriter<Box<dyn Write>>,
            Option<BufWriter<Box<dyn Write>>>,
        ) -> Result<(), String>,
    >,
);

#[allow(clippy::type_complexity)]
struct LocationWriter(pub Box<dyn Fn(File, BufWriter<Box<dyn Write>>) -> Result<(), String>>);

#[allow(clippy::type_complexity)]
type FormatMap<'a> = HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter, LocationWriter)>;

fn add_scientific(formats: &mut FormatMap, matches: &ArgMatches, random: Arc<dyn Random + Send + Sync>) {
    if cfg!(feature = "scientific-format") {
        use vrp_scientific::lilim::{LilimProblem, LilimSolution};
        use vrp_scientific::solomon::read_init_solution as read_init_solomon;
        use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};

        let is_rounded = matches.is_present(ROUNDED_ARG_NAME);

        formats.insert(
            "solomon",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_solomon(is_rounded)
                })),
                InitSolutionReader(Box::new(move |file, problem| {
                    read_init_solomon(BufReader::new(file), problem, random.clone())
                })),
                SolutionWriter(Box::new(|_, solution, cost, _, writer, _| (&solution, cost).write_solomon(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
        formats.insert(
            "lilim",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_lilim(is_rounded)
                })),
                InitSolutionReader(Box::new(|_file, _problem| unimplemented!())),
                SolutionWriter(Box::new(|_, solution, cost, _, writer, _| (&solution, cost).write_lilim(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
        formats.insert(
            "tsplib",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_tsplib(is_rounded)
                })),
                InitSolutionReader(Box::new(|_file, _problem| unimplemented!())),
                SolutionWriter(Box::new(|_, solution, cost, _, writer, _| (&solution, cost).write_tsplib(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
    }
}

fn add_pragmatic(formats: &mut FormatMap, random: Arc<dyn Random + Send + Sync>) {
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
            InitSolutionReader(Box::new(move |file, problem| {
                read_init_pragmatic(BufReader::new(file), problem, random.clone())
            })),
            SolutionWriter(Box::new(|problem, solution, cost, metrics, default_writer, geojson_writer| {
                geojson_writer
                    .map_or(Ok(()), |geojson_writer| (&solution, cost).write_geo_json(problem, geojson_writer))
                    .and_then(|_| {
                        if let Some(metrics) = metrics {
                            (&solution, cost, &metrics).write_pragmatic_json(problem, default_writer)
                        } else {
                            (&solution, cost).write_pragmatic_json(problem, default_writer)
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

fn get_formats<'a>(matches: &ArgMatches, random: Arc<dyn Random + Send + Sync>) -> FormatMap<'a> {
    let mut formats = FormatMap::default();

    add_scientific(&mut formats, matches, random.clone());
    add_pragmatic(&mut formats, random);

    formats
}

pub fn get_solve_app() -> Command<'static> {
    Command::new("solve")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(
            Arg::new(FORMAT_ARG_NAME)
                .help("Specifies the problem type")
                .required(true)
                .possible_values(&["solomon", "lilim", "tsplib", "pragmatic"])
                .index(1),
        )
        .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
        .arg(
            Arg::new(GENERATIONS_ARG_NAME)
                .help("Specifies maximum number of generations")
                .short('n')
                .long(GENERATIONS_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(TIME_ARG_NAME)
                .help("Specifies max time algorithm run in seconds")
                .short('t')
                .long(TIME_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(MIN_CV_ARG_NAME)
                .help(
                    "Specifies variation coefficient termination criteria in form \"type,sample_size,threshold,is_global\"",
                )
                .short('v')
                .long(MIN_CV_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(INIT_SOLUTION_ARG_NAME)
                .help("Specifies path to file with initial solution")
                .short('i')
                .long(INIT_SOLUTION_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(INIT_SIZE_ARG_NAME)
                .help("Specifies amount of initial solutions. Min is 1")
                .long(INIT_SIZE_ARG_NAME)
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
                .help("Specifies path to file for result output")
                .short('o')
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(GET_LOCATIONS_ARG_NAME)
                .help("Returns list of unique locations")
                .short('l')
                .long(GET_LOCATIONS_ARG_NAME)
                .required(false),
        )
        .arg(
            Arg::new(GEO_JSON_ARG_NAME)
                .help("Specifies path to solution output in geo json format")
                .short('g')
                .long(GEO_JSON_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(CONFIG_ARG_NAME)
                .help("Specifies path to algorithm configuration file")
                .short('c')
                .long(CONFIG_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(LOG_ARG_NAME)
                .help("Specifies whether default logging is enabled")
                .long(LOG_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::new(CHECK_ARG_NAME)
                .help("Specifies whether final solution should be checked for feasibility")
                .long(CHECK_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::new(SEARCH_MODE_ARG_NAME)
                .help("Specifies solution space search mode")
                .long(SEARCH_MODE_ARG_NAME)
                .short('s')
                .required(false)
                .possible_values(&["broad", "deep"])
                .default_value("broad"),
        )
        .arg(
            Arg::new(PARALLELISM_ARG_NAME)
                .help("Specifies data parallelism settings in format \"num_thread_pools,threads_per_pool\"")
                .long(PARALLELISM_ARG_NAME)
                .short('p')
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new(HEURISTIC_ARG_NAME)
                .help("Specifies hyper heuristic algorithm")
                .long(HEURISTIC_ARG_NAME)
                .short('e')
                .required(false)
                .possible_values(&["default", "dynamic", "static"])
                .default_value("default"),
        )
        .arg(
            Arg::new(EXPERIMENTAL_ARG_NAME)
                .help("Specifies whether experimental (unstable) features are enabled.")
                .long(EXPERIMENTAL_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::new(ROUNDED_ARG_NAME)
                .help("Specifies whether costs are rounded. Applicable only for scientific formats.")
                .long(ROUNDED_ARG_NAME)
                .required(false)
                .takes_value(false),
        )
}

/// Runs solver commands.
pub fn run_solve(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> Result<(), String> {
    let max_time = parse_int_value::<usize>(matches, TIME_ARG_NAME, "max time")?;

    let environment = get_environment(matches, max_time)?;

    let formats = get_formats(matches, environment.random.clone());

    // required
    let problem_path = matches.value_of(PROBLEM_ARG_NAME).unwrap();
    let problem_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let problem_file = open_file(problem_path, "problem");

    // optional
    let max_generations = parse_int_value::<usize>(matches, GENERATIONS_ARG_NAME, "max generations")?;
    let telemetry_mode = if matches.is_present(LOG_ARG_NAME) {
        get_default_telemetry_mode(environment.logger.clone())
    } else {
        TelemetryMode::None
    };

    let is_check_requested = matches.is_present(CHECK_ARG_NAME);
    let min_cv = get_min_cv(matches)?;
    let init_solution = matches.value_of(INIT_SOLUTION_ARG_NAME).map(|path| open_file(path, "init solution"));
    let init_size = get_init_size(matches)?;
    let config = matches.value_of(CONFIG_ARG_NAME).map(|path| open_file(path, "config"));
    let matrix_files = get_matrix_files(matches);
    let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out solution"));
    let out_geojson = matches.value_of(GEO_JSON_ARG_NAME).map(|path| create_file(path, "out geojson"));
    let is_get_locations_set = matches.is_present(GET_LOCATIONS_ARG_NAME);
    let mode = matches.value_of(SEARCH_MODE_ARG_NAME);

    match formats.get(problem_format) {
        Some((problem_reader, init_reader, solution_writer, locations_writer)) => {
            let out_buffer = out_writer_func(out_result);
            let geo_buffer = out_geojson.map(|geojson| create_write_buffer(Some(geojson)));

            if is_get_locations_set {
                locations_writer.0(problem_file, out_buffer).map_err(|err| format!("cannot get locations '{}'", err))
            } else {
                match problem_reader.0(problem_file, matrix_files) {
                    Ok(problem) => {
                        let problem = Arc::new(problem);
                        let solutions = init_solution
                            .map(|file| {
                                init_reader.0(file, problem.clone())
                                    .map_err(|err| format!("cannot read initial solution '{}'", err))
                                    .map(|solution| {
                                        vec![InsertionContext::new_from_solution(
                                            problem.clone(),
                                            (solution, None),
                                            environment.clone(),
                                        )]
                                    })
                            })
                            .unwrap_or_else(|| Ok(Vec::new()))?;

                        let solver = if let Some(config) = config {
                            create_builder_from_config_file(problem.clone(), solutions, BufReader::new(config))
                                .and_then(|builder| builder.build())
                                .map(|config| Solver::new(problem.clone(), config))
                                .map_err(|err| format!("cannot read config: '{}'", err))?
                        } else {
                            let config = create_default_config_builder(
                                problem.clone(),
                                environment.clone(),
                                telemetry_mode.clone(),
                            )
                            .with_init_solutions(solutions, init_size)
                            .with_max_generations(max_generations)
                            .with_max_time(max_time)
                            .with_min_cv(min_cv, "min_cv".to_string())
                            .with_context(RefinementContext::new(
                                problem.clone(),
                                get_population(mode, problem.objective.clone(), environment.clone()),
                                telemetry_mode,
                                environment.clone(),
                            ))
                            .with_heuristic(get_heuristic(matches, problem.clone(), environment)?)
                            .build()?;

                            Solver::new(problem.clone(), config)
                        };

                        let (solution, cost, metrics) =
                            solver.solve().map_err(|err| format!("cannot find any solution: '{}'", err))?;

                        solution_writer.0(&problem, solution, cost, metrics, out_buffer, geo_buffer).unwrap();

                        if is_check_requested {
                            check_pragmatic_solution_with_args(matches)?;
                            println!("solution feasibility check is completed successfully");
                        }

                        Ok(())
                    }
                    Err(error) => {
                        Err(format!("cannot read {} problem from '{}': '{}'", problem_format, problem_path, error))
                    }
                }
            }
        }
        None => Err(format!("unknown format: '{}'", problem_format)),
    }
}

fn get_min_cv(matches: &ArgMatches) -> Result<Option<(String, usize, f64, bool)>, String> {
    let err_result = Err("cannot parse min_cv parameter".to_string());
    matches
        .value_of(MIN_CV_ARG_NAME)
        .map(|arg| match arg.split(',').collect::<Vec<_>>().as_slice() {
            [cv_type, sample, threshold, is_global] => {
                match (*cv_type, sample.parse::<usize>(), threshold.parse::<f64>(), is_global.parse::<bool>()) {
                    (cv_type, Ok(sample), Ok(threshold), Ok(is_global))
                        if cv_type == "sample" || cv_type == "period" =>
                    {
                        Ok(Some((cv_type.to_string(), sample, threshold, is_global)))
                    }
                    _ => err_result,
                }
            }
            _ => err_result,
        })
        .unwrap_or(Ok(None))
}

fn get_init_size(matches: &ArgMatches) -> Result<Option<usize>, String> {
    matches
        .value_of(INIT_SIZE_ARG_NAME)
        .map(|size| {
            if let Some(value) = size.parse::<usize>().ok().and_then(|value| if value < 1 { None } else { Some(value) })
            {
                Ok(Some(value))
            } else {
                Err(format!("init size must be an integer bigger than 0, got '{}'", size))
            }
        })
        .unwrap_or(Ok(None))
}

fn get_environment(matches: &ArgMatches, max_time: Option<usize>) -> Result<Arc<Environment>, String> {
    let quota = Some(create_interruption_quota(max_time));
    let is_experimental = matches.is_present(EXPERIMENTAL_ARG_NAME);

    matches
        .value_of(PARALLELISM_ARG_NAME)
        .map(|arg| {
            if let [num_thread_pools, threads_per_pool] =
                arg.split(',').filter_map(|line| line.parse::<usize>().ok()).collect::<Vec<_>>().as_slice()
            {
                let parallelism = Parallelism::new(*num_thread_pools, *threads_per_pool);
                let logger: InfoLogger = if matches.is_present(LOG_ARG_NAME) {
                    Arc::new(|msg: &str| println!("{}", msg))
                } else {
                    Arc::new(|_: &str| {})
                };
                Ok(Arc::new(Environment::new(
                    Arc::new(DefaultRandom::default()),
                    quota.clone(),
                    parallelism,
                    logger,
                    is_experimental,
                )))
            } else {
                Err("cannot parse parallelism parameter".to_string())
            }
        })
        .unwrap_or_else(|| Ok(Arc::new(Environment { quota, is_experimental, ..Environment::default() })))
}

fn get_matrix_files(matches: &ArgMatches) -> Option<Vec<File>> {
    matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| open_file(path, "routing matrix")).collect())
}

fn get_population(
    mode: Option<&str>,
    objective: Arc<ProblemObjective>,
    environment: Arc<Environment>,
) -> TargetPopulation {
    let selection_size = get_default_selection_size(environment.as_ref());

    match mode {
        Some("deep") => Box::new(ElitismPopulation::new(objective, environment.random.clone(), 4, selection_size)),
        _ => get_default_population(objective, environment, selection_size),
    }
}

fn get_heuristic(
    matches: &ArgMatches,
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> Result<TargetHeuristic, String> {
    match matches.value_of(HEURISTIC_ARG_NAME) {
        Some("dynamic") => Ok(get_dynamic_heuristic(problem, environment)),
        Some("static") => Ok(get_static_heuristic(problem, environment)),
        Some(name) if name != "default" => Err(format!("unknown heuristic type name: '{}'", name)),
        _ => Ok(get_default_heuristic(problem, environment)),
    }
}

fn check_pragmatic_solution_with_args(matches: &ArgMatches) -> Result<(), String> {
    check_solution(matches, "pragmatic", PROBLEM_ARG_NAME, OUT_RESULT_ARG_NAME, MATRIX_ARG_NAME)
}

/// Creates interruption quota.
pub fn create_interruption_quota(max_time: Option<usize>) -> Arc<dyn Quota + Send + Sync> {
    struct InterruptionQuota {
        inner: Option<Arc<dyn Quota + Send + Sync>>,
        should_interrupt: Arc<AtomicBool>,
    }

    impl Quota for InterruptionQuota {
        fn is_reached(&self) -> bool {
            self.inner.as_ref().map_or(false, |inner| inner.is_reached())
                || self.should_interrupt.load(Ordering::Relaxed)
        }
    }

    let inner = max_time.map::<Arc<dyn Quota + Send + Sync>, _>(|time| Arc::new(TimeQuota::new(time as f64)));
    let should_interrupt = Arc::new(AtomicBool::new(false));

    // NOTE ignore error which happens in unit tests
    let _ = ctrlc::set_handler({
        let should_interrupt = should_interrupt.clone();
        move || {
            should_interrupt.store(true, Ordering::Relaxed);
        }
    });

    Arc::new(InterruptionQuota { inner, should_interrupt })
}
