extern crate clap;

use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Error};
use std::ops::Deref;
use std::process;

use clap::{App, Arg, ArgMatches};
use core::models::{Problem, Solution};

mod solver;
pub use self::solver::Solver;
use crate::solver::SolverBuilder;
use scientific::common::read_init_solution;
use scientific::lilim::{write_lilim_solution, LilimProblem};
use scientific::solomon::{write_solomon_solution, SolomonProblem};
use std::sync::Arc;

struct ProblemReader(Box<dyn Fn(File) -> Result<Problem, String>>);

struct InitSolutionReader(Box<dyn Fn(File, Arc<Problem>) -> Option<Solution>>);

struct SolutionWriter(Box<dyn Fn(Solution) -> Result<(), Error>>);

fn get_formats<'a>() -> HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter)> {
    vec![
        (
            "solomon",
            (
                ProblemReader(Box::new(|file: File| file.parse_solomon())),
                InitSolutionReader(Box::new(|file, problem| read_init_solution(BufReader::new(file), problem).ok())),
                SolutionWriter(Box::new(|solution: Solution| {
                    write_solomon_solution(BufWriter::new(Box::new(stdout())), &solution)
                })),
            ),
        ),
        (
            "lilim",
            (
                ProblemReader(Box::new(|file: File| file.parse_lilim())),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|solution: Solution| {
                    write_lilim_solution(BufWriter::new(Box::new(stdout())), &solution)
                })),
            ),
        ),
    ]
    .into_iter()
    .collect()
}

fn get_matches(formats: Vec<&str>) -> ArgMatches {
    App::new("Vehicle Routing Problem Solver")
        .version("0.1")
        .author("Ilya Builuk <ilya.builuk@gmail.com>")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(Arg::with_name("PROBLEM").help("Sets the problem file to use").required(true).index(1))
        .arg(
            Arg::with_name("FORMAT")
                .help("Specifies the problem type")
                .required(true)
                .possible_values(formats.as_slice())
                .index(2),
        )
        .arg(
            Arg::with_name("max-generations")
                .help("Specifies maximum amount of generations")
                .short("g")
                .long("max-generations")
                .required(false)
                .default_value("2000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("variation-coefficient")
                .help("Specifies variation-coefficient termination criteria in form \"sample_size,threshold\"")
                .short("v")
                .long("variation-coefficient")
                .required(false)
                .default_value("200,0.01")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("minimize-routes")
                .help("Prefer less routes over total cost")
                .short("r")
                .long("minimize-routes")
                .required(false)
                .default_value("true")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("init-solution")
                .help("Specifies path to file with initial solution")
                .short("s")
                .long("init-solution")
                .required(false)
                .takes_value(true),
        )
        .get_matches()
}

fn main() {
    let formats = get_formats();
    let matches = get_matches(formats.keys().map(|s| s.deref()).collect::<Vec<&str>>());

    // required
    let problem_path = matches.value_of("PROBLEM").unwrap();
    let problem_format = matches.value_of("FORMAT").unwrap();
    let input_file = File::open(problem_path).unwrap_or_else(|err| {
        eprintln!("Cannot open problem file '{}': '{}'", problem_path, err.to_string());
        process::exit(1);
    });

    // optional
    let max_generations = matches.value_of("max-generations").unwrap().parse::<usize>().unwrap_or_else(|err| {
        eprintln!("Cannot get max-generations: '{}'", err.to_string());
        process::exit(1);
    });
    let variation_coefficient = matches
        .value_of("variation-coefficient")
        .unwrap()
        .split(",")
        .map(|line| {
            line.parse::<f64>().unwrap_or_else(|err| {
                eprintln!("Cannot get variation-coefficient: '{}'", err.to_string());
                process::exit(1);
            })
        })
        .collect();
    let minimize_routes = matches.value_of("minimize-routes").unwrap().parse::<bool>().unwrap_or_else(|err| {
        eprintln!("Cannot get minimize-routes: '{}'", err.to_string());
        process::exit(1);
    });
    let init_solution = matches.value_of("init-solution").and_then(|path| {
        Some(File::open(path).unwrap_or_else(|err| {
            eprintln!("Cannot open init solution file '{}': '{}'", problem_path, err.to_string());
            process::exit(1);
        }))
    });

    match formats.get(problem_format) {
        Some((problem_reader, init_reader, solution_writer)) => {
            let solution = match problem_reader.0(input_file) {
                Ok(problem) => {
                    let problem = Arc::new(problem);
                    let solution = init_solution.and_then(|file| init_reader.0(file, problem.clone()));
                    SolverBuilder::new()
                        .with_init_solution(solution.and_then(|s| Some((problem.clone(), Arc::new(s)))))
                        .with_minimize_routes(minimize_routes)
                        .with_max_generations(max_generations)
                        .with_variation_coefficient(variation_coefficient)
                        .build()
                        .solve(problem)
                }
                Err(error) => {
                    eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                    process::exit(1);
                }
            };

            match solution {
                Some(solution) => solution_writer.0(solution.0).unwrap(),
                None => println!("Cannot find any solution"),
            };
        }
        None => {
            eprintln!("Unknown format: '{}'", problem_format);
            process::exit(1);
        }
    }
}
