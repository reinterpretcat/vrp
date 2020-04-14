use super::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::process;
use std::sync::Arc;
use vrp_cli::{get_errors_serialized, get_locations_serialized};
use vrp_core::models::{Problem, Solution};
use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::PragmaticSolution;
use vrp_scientific::common::read_init_solution;
use vrp_scientific::lilim::{LilimProblem, LilimSolution};
use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};
use vrp_solver::SolverBuilder;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const GENERATIONS_ARG_NAME: &str = "max-generations";
const TIME_ARG_NAME: &str = "max-time";
const GEO_JSON_ARG_NAME: &str = "geo-json";

const INIT_SOLUTION_ARG_NAME: &str = "init-solution";
const OUT_RESULT_ARG_NAME: &str = "out-result";
const GET_LOCATIONS_ARG_NAME: &str = "get-locations";

struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Option<Solution>>);

struct SolutionWriter(
    pub  Box<
        dyn Fn(&Problem, Solution, BufWriter<Box<dyn Write>>, Option<BufWriter<Box<dyn Write>>>) -> Result<(), String>,
    >,
);

struct LocationWriter(pub Box<dyn Fn(File, BufWriter<Box<dyn Write>>) -> Result<(), String>>);

fn get_formats<'a>() -> HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter, LocationWriter)> {
    vec![
        (
            "solomon",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_solomon()
                })),
                InitSolutionReader(Box::new(|file, problem| read_init_solution(BufReader::new(file), problem).ok())),
                SolutionWriter(Box::new(|_, solution, writer, _| solution.write_solomon(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        ),
        (
            "lilim",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_lilim()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|_, solution, writer, _| solution.write_lilim(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        ),
        (
            "pragmatic",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    if let Some(matrices) = matrices {
                        let matrices = matrices.into_iter().map(|m| BufReader::new(m)).collect();
                        (BufReader::new(problem), matrices).read_pragmatic()
                    } else {
                        println!("configured to use single approximated routing matrix");
                        BufReader::new(problem).read_pragmatic()
                    }
                    .map_err(|errors| errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join("\t\n"))
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|problem, solution, default_writer, geojson_writer| {
                    geojson_writer
                        .map_or(Ok(()), |geojson_writer| solution.write_geo_json(problem, geojson_writer))
                        .and_then(|_| solution.write_pragmatic_json(problem, default_writer))
                })),
                LocationWriter(Box::new(|problem, writer| {
                    let mut writer = writer;
                    deserialize_problem(BufReader::new(problem))
                        .map_err(|errors| get_errors_serialized(&errors))
                        .and_then(|problem| get_locations_serialized(&problem))
                        .and_then(|locations| writer.write_all(locations.as_bytes()).map_err(|err| err.to_string()))
                })),
            ),
        ),
    ]
    .into_iter()
    .collect()
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
}

/// Runs solver commands.
pub fn run_solve(matches: &ArgMatches) {
    let formats = get_formats();

    // required
    let problem_path = matches.value_of(PROBLEM_ARG_NAME).unwrap();
    let problem_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let problem_file = open_file(problem_path, "problem");

    // optional
    let max_generations = matches.value_of(GENERATIONS_ARG_NAME).map(|arg| {
        arg.parse::<usize>().unwrap_or_else(|err| {
            eprintln!("Cannot get max generations: '{}'", err.to_string());
            process::exit(1);
        })
    });
    let max_time = matches.value_of(TIME_ARG_NAME).map(|arg| {
        arg.parse::<usize>().unwrap_or_else(|err| {
            eprintln!("Cannot get max time: '{}'", err.to_string());
            process::exit(1);
        })
    });
    let init_solution = matches.value_of(INIT_SOLUTION_ARG_NAME).map(|path| open_file(path, "init solution"));
    let matrix_files = matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| open_file(path, "routing matrix")).collect());
    let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out solution"));
    let out_geojson = matches.value_of(GEO_JSON_ARG_NAME).map(|path| create_file(path, "out geojson"));
    let is_get_locations_set = matches.is_present(GET_LOCATIONS_ARG_NAME);

    match formats.get(problem_format) {
        Some((problem_reader, init_reader, solution_writer, locations_writer)) => {
            let out_buffer = create_write_buffer(out_result);
            let geo_buffer = out_geojson.map(|geojson| create_write_buffer(Some(geojson)));

            if is_get_locations_set {
                locations_writer.0(problem_file, out_buffer).unwrap_or_else(|err| {
                    eprintln!("Cannot get locations '{}'", err);
                    process::exit(1);
                });
            } else {
                match problem_reader.0(problem_file, matrix_files) {
                    Ok(problem) => {
                        let problem = Arc::new(problem);
                        let solution = init_solution.and_then(|file| init_reader.0(file, problem.clone()));
                        let solution = SolverBuilder::default()
                            .with_init_solution(solution.map(|s| (problem.clone(), Arc::new(s))))
                            .with_max_generations(max_generations)
                            .with_max_time(max_time)
                            .build()
                            .solve(problem.clone());
                        match solution {
                            Some(solution) => solution_writer.0(&problem, solution.0, out_buffer, geo_buffer).unwrap(),
                            None => println!("Cannot find any solution"),
                        };
                    }
                    Err(error) => {
                        eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                        process::exit(1);
                    }
                };
            }
        }
        None => {
            eprintln!("Unknown format: '{}'", problem_format);
            process::exit(1);
        }
    }
}
