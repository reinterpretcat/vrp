#[cfg(test)]
#[path = "../../tests/unit/commands/solve_test.rs"]
mod solve_test;

use super::*;

use clap::ArgAction;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vrp_cli::core::solver::TargetHeuristic;
use vrp_cli::extensions::solve::config::create_builder_from_config_file;
use vrp_cli::extensions::solve::formats::*;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::GoalContext;
use vrp_core::prelude::*;
use vrp_core::rosomaxa::{evolution::*, get_default_population, get_default_selection_size};
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

pub fn get_solve_app() -> Command {
    Command::new("solve")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(
            Arg::new(FORMAT_ARG_NAME)
                .help("Specifies the problem type")
                .required(true)
                .value_parser(["solomon", "lilim", "tsplib", "pragmatic"])
                .index(1),
        )
        .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
        .arg(
            Arg::new(GENERATIONS_ARG_NAME)
                .help("Specifies maximum number of generations")
                .short('n')
                .long(GENERATIONS_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(TIME_ARG_NAME)
                .help("Specifies max time algorithm run in seconds")
                .short('t')
                .long(TIME_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(MIN_CV_ARG_NAME)
                .help(
                    "Specifies variation coefficient termination criteria in form \"type,sample_size,threshold,is_global\"",
                )
                .short('v')
                .long(MIN_CV_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(INIT_SOLUTION_ARG_NAME)
                .help("Specifies path to file with initial solution")
                .short('i')
                .long(INIT_SOLUTION_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(INIT_SIZE_ARG_NAME)
                .help("Specifies amount of initial solutions. Min is 1")
                .long(INIT_SIZE_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(MATRIX_ARG_NAME)
                .help("Specifies path to file with routing matrix")
                .short('m')
                .long(MATRIX_ARG_NAME)
                .num_args(1..)
                .required(false)
        )
        .arg(
            Arg::new(OUT_RESULT_ARG_NAME)
                .help("Specifies path to file for result output")
                .short('o')
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(GET_LOCATIONS_ARG_NAME)
                .help("Returns list of unique locations")
                .short('l')
                .long(GET_LOCATIONS_ARG_NAME)
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(GEO_JSON_ARG_NAME)
                .help("Specifies path to solution output in geo json format")
                .short('g')
                .long(GEO_JSON_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(CONFIG_ARG_NAME)
                .help("Specifies path to algorithm configuration file")
                .short('c')
                .long(CONFIG_ARG_NAME)
                .required(false)
        )
        .arg(
            Arg::new(LOG_ARG_NAME)
                .help("Specifies whether default logging is enabled")
                .long(LOG_ARG_NAME)
                .required(false)
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(CHECK_ARG_NAME)
                .help("Specifies whether final solution should be checked for feasibility")
                .long(CHECK_ARG_NAME)
                .required(false)
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(SEARCH_MODE_ARG_NAME)
                .help("Specifies solution space search mode")
                .long(SEARCH_MODE_ARG_NAME)
                .short('s')
                .required(false)
                .value_parser(["broad", "deep"])
                .default_value("broad"),
        )
        .arg(
            Arg::new(PARALLELISM_ARG_NAME)
                .help("Specifies data parallelism settings in format \"num_thread_pools,threads_per_pool\"")
                .long(PARALLELISM_ARG_NAME)
                .short('p')
                .required(false)
        )
        .arg(
            Arg::new(HEURISTIC_ARG_NAME)
                .help("Specifies hyper heuristic algorithm")
                .long(HEURISTIC_ARG_NAME)
                .short('e')
                .required(false)
                .value_parser(["default", "dynamic", "static"])
                .default_value("default"),
        )
        .arg(
            Arg::new(EXPERIMENTAL_ARG_NAME)
                .help("Specifies whether experimental (unstable) features are enabled.")
                .long(EXPERIMENTAL_ARG_NAME)
                .required(false)
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new(ROUNDED_ARG_NAME)
                .help("Specifies whether costs are rounded. Applicable only for scientific formats.")
                .long(ROUNDED_ARG_NAME)
                .required(false)
                .action(ArgAction::SetTrue)
        )
}

/// Runs solver commands.
pub fn run_solve(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> Result<(), GenericError> {
    let environment = get_environment(matches)?;

    let is_rounded = matches.get_one::<bool>(ROUNDED_ARG_NAME).copied().unwrap_or(false);
    let formats = get_formats(is_rounded, environment.random.clone());

    let problem_path = matches
        .get_one::<String>(PROBLEM_ARG_NAME)
        .ok_or_else(|| GenericError::from(format!("{PROBLEM_ARG_NAME} must be set")))?;
    let problem_format = matches
        .get_one::<String>(FORMAT_ARG_NAME)
        .ok_or_else(|| GenericError::from(format!("{FORMAT_ARG_NAME} must be set")))?;

    let problem_file = open_file(problem_path, "problem");

    let init_solution = matches.get_one::<String>(INIT_SOLUTION_ARG_NAME).map(|path| open_file(path, "init solution"));
    let config = matches.get_one::<String>(CONFIG_ARG_NAME).map(|path| open_file(path, "config"));
    let matrix_files = get_matrix_files(matches);
    let out_result = matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out solution"));
    let out_geojson = matches.get_one::<String>(GEO_JSON_ARG_NAME).map(|path| create_file(path, "out geojson"));

    let is_get_locations_set = matches.get_one::<bool>(GET_LOCATIONS_ARG_NAME).copied().unwrap_or(false);
    let is_check_requested = matches.get_one::<bool>(CHECK_ARG_NAME).copied().unwrap_or(false);

    match formats.get(problem_format.as_str()) {
        Some((
            ProblemReader(problem_reader),
            init_reader,
            SolutionWriter(solution_writer),
            LocationWriter(locations_writer),
        )) => {
            let out_buffer = out_writer_func(out_result);
            let geo_buffer = out_geojson.map(|geojson| create_write_buffer(Some(geojson)));

            if is_get_locations_set {
                locations_writer(problem_file, out_buffer).map_err(|err| format!("cannot get locations '{err}'").into())
            } else {
                match problem_reader(problem_file, matrix_files) {
                    Ok(problem) => {
                        let problem = Arc::new(problem);
                        let init_solutions = init_solution
                            .map(|file| read_init_solution(problem.clone(), environment.clone(), file, init_reader))
                            .unwrap_or_else(|| Ok(Vec::default()))?;

                        let solver = if let Some(config) = config {
                            from_config_parameters(problem.clone(), init_solutions, config)?
                        } else {
                            from_cli_parameters(problem.clone(), environment, init_solutions, matches)?
                        };

                        let solution = solver.solve().map_err(|err| format!("cannot find any solution: '{err}'"))?;

                        solution_writer(&problem, solution, out_buffer, geo_buffer)?;

                        if is_check_requested {
                            check_pragmatic_solution_with_args(matches)?;
                            println!("solution feasibility check is completed successfully");
                        }

                        Ok(())
                    }
                    Err(error) => {
                        Err(format!("cannot read {problem_format} problem from '{problem_path}': '{error}'").into())
                    }
                }
            }
        }
        None => Err(format!("unknown format: '{problem_format}'").into()),
    }
}

fn read_init_solution(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    file: File,
    InitSolutionReader(init_reader): &InitSolutionReader,
) -> GenericResult<Vec<InsertionContext>> {
    init_reader(file, problem.clone())
        .map_err(|err| format!("cannot read initial solution '{err}'").into())
        .map(|solution| vec![InsertionContext::new_from_solution(problem.clone(), (solution, None), environment)])
}

fn from_config_parameters(
    problem: Arc<Problem>,
    init_solutions: Vec<InsertionContext>,
    config: File,
) -> GenericResult<Solver> {
    create_builder_from_config_file(problem.clone(), init_solutions, BufReader::new(config))
        .and_then(|builder| builder.build())
        .map(|config| Solver::new(problem.clone(), config))
        .map_err(|err| format!("cannot read config: '{err}'").into())
}

fn from_cli_parameters(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    init_solutions: Vec<InsertionContext>,
    matches: &ArgMatches,
) -> GenericResult<Solver> {
    let max_time = parse_int_value::<usize>(matches, TIME_ARG_NAME, "max time")?;

    let max_generations = parse_int_value::<usize>(matches, GENERATIONS_ARG_NAME, "max generations")?;
    let telemetry_mode = if matches.get_one::<bool>(LOG_ARG_NAME).copied().unwrap_or(false) {
        get_default_telemetry_mode(environment.logger.clone())
    } else {
        TelemetryMode::None
    };
    let min_cv = get_min_cv(matches)?;
    let init_size = get_init_size(matches)?;
    let mode = matches.get_one::<String>(SEARCH_MODE_ARG_NAME);

    let config = VrpConfigBuilder::new(problem.clone())
        .set_environment(environment.clone())
        .set_telemetry_mode(telemetry_mode.clone())
        .prebuild()?
        .with_init_solutions(init_solutions, init_size)
        .with_max_generations(max_generations)
        .with_max_time(max_time)
        .with_min_cv(min_cv, "min_cv".to_string())
        .with_context(RefinementContext::new(
            problem.clone(),
            get_population(mode, problem.goal.clone(), environment.clone()),
            telemetry_mode,
            environment.clone(),
        ))
        .with_heuristic(get_heuristic(matches, problem.clone(), environment)?)
        .build()?;

    Ok(Solver::new(problem.clone(), config))
}

fn get_min_cv(matches: &ArgMatches) -> GenericResult<Option<(String, usize, Float, bool)>> {
    let err_result = Err("cannot parse min_cv parameter".into());
    matches
        .get_one::<String>(MIN_CV_ARG_NAME)
        .map(|arg| match arg.split(',').collect::<Vec<_>>().as_slice() {
            [cv_type, sample, threshold, is_global] => {
                match (*cv_type, sample.parse::<usize>(), threshold.parse::<Float>(), is_global.parse::<bool>()) {
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

fn get_init_size(matches: &ArgMatches) -> GenericResult<Option<usize>> {
    matches
        .get_one::<String>(INIT_SIZE_ARG_NAME)
        .map(|size| {
            if let Some(value) = size.parse::<usize>().ok().and_then(|value| if value < 1 { None } else { Some(value) })
            {
                Ok(Some(value))
            } else {
                Err(format!("init size must be an integer bigger than 0, got '{size}'").into())
            }
        })
        .unwrap_or(Ok(None))
}

fn get_environment(matches: &ArgMatches) -> GenericResult<Arc<Environment>> {
    let max_time = parse_int_value::<usize>(matches, TIME_ARG_NAME, "max time")?;
    let quota = Some(create_interruption_quota(max_time));
    let is_experimental = matches.get_one::<bool>(EXPERIMENTAL_ARG_NAME).copied().unwrap_or(false);

    matches
        .get_one::<String>(PARALLELISM_ARG_NAME)
        .map(|arg| {
            if let [num_thread_pools, threads_per_pool] =
                arg.split(',').filter_map(|line| line.parse::<usize>().ok()).collect::<Vec<_>>().as_slice()
            {
                let parallelism = Parallelism::new(*num_thread_pools, *threads_per_pool);
                let logger: InfoLogger = if matches.get_one::<bool>(LOG_ARG_NAME).copied().unwrap_or(false) {
                    Arc::new(|msg: &str| println!("{msg}"))
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
                Err("cannot parse parallelism parameter".into())
            }
        })
        .unwrap_or_else(|| Ok(Arc::new(Environment { quota, is_experimental, ..Environment::default() })))
}

fn get_matrix_files(matches: &ArgMatches) -> Option<Vec<File>> {
    matches
        .get_many::<String>(MATRIX_ARG_NAME)
        .map(|paths| paths.map(|path| open_file(path, "routing matrix")).collect())
}

fn get_population(
    mode: Option<&String>,
    objective: Arc<GoalContext>,
    environment: Arc<Environment>,
) -> TargetPopulation {
    let selection_size = get_default_selection_size(environment.as_ref());

    match mode.map(String::as_str) {
        Some("deep") => Box::new(ElitismPopulation::new(objective, environment.random.clone(), 4, selection_size)),
        _ => get_default_population(objective, environment, selection_size),
    }
}

fn get_heuristic(
    matches: &ArgMatches,
    problem: Arc<Problem>,
    environment: Arc<Environment>,
) -> GenericResult<TargetHeuristic> {
    match matches.get_one::<String>(HEURISTIC_ARG_NAME).map(String::as_str) {
        Some("dynamic") => Ok(Box::new(get_dynamic_heuristic(problem, environment))),
        Some("static") => Ok(Box::new(get_static_heuristic(problem, environment))),
        Some(name) if name != "default" => Err(format!("unknown heuristic type name: '{name}'").into()),
        _ => Ok(get_default_heuristic(problem, environment)),
    }
}

fn check_pragmatic_solution_with_args(matches: &ArgMatches) -> GenericResult<()> {
    check_solution(matches, "pragmatic", PROBLEM_ARG_NAME, OUT_RESULT_ARG_NAME, MATRIX_ARG_NAME)
}

/// Creates interruption quota.
pub fn create_interruption_quota(max_time: Option<usize>) -> Arc<dyn Quota> {
    struct InterruptionQuota {
        inner: Option<Arc<dyn Quota>>,
        should_interrupt: Arc<AtomicBool>,
    }

    impl Quota for InterruptionQuota {
        fn is_reached(&self) -> bool {
            self.inner.as_ref().map_or(false, |inner| inner.is_reached())
                || self.should_interrupt.load(Ordering::Relaxed)
        }
    }

    let inner = max_time.map::<Arc<dyn Quota>, _>(|time| Arc::new(TimeQuota::new(time as Float)));
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
