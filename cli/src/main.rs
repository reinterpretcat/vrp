//! A command line interface to solve variations of *Vehicle Routing Problem*.
//!
//! ## Usage
//!
//! Depending on your problem type and settings, you might need to specify different command line
//! arguments, for example:
//!
//! - solve scientific problem from **solomon** set using existing solution
//!
//!     `cli solomon RC1_10_1.txt --init-solution RC1_10_1_solution.txt  --max-time=3600`
//!
//! - solve custom problem specified in **pragmatic** json format with its routing matrix.
//!
//!     `cli pragmatic problem_definition.json -m routing_matrix.json --max-generations=1000`
//!
//! - solve scientific problem from **li lim** set writing solution to the file specified
//!
//!     `cli lilim LC1_10_2.txt -o LC1_10_2_solution.txt`
//!
//! For more details, simply run
//!
//!     cli --help

mod args;

use self::args::*;

mod formats;

use self::formats::*;

use std::fs::File;
use std::ops::Deref;
use std::process;

use clap::Values;
use solver::SolverBuilder;
use std::io::{stdout, BufWriter, Write};
use std::sync::Arc;

fn main() {
    let formats = get_formats();
    let matches = get_arg_matches(formats.keys().map(|s| s.deref()).collect::<Vec<&str>>());

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
        arg.parse::<f64>().unwrap_or_else(|err| {
            eprintln!("Cannot get max time: '{}'", err.to_string());
            process::exit(1);
        })
    });
    let variation_coefficient = matches.value_of(VARIATION_COEFFICIENT_ARG_NAME).map(|args| {
        args.split(',')
            .map(|line| {
                line.parse::<f64>().unwrap_or_else(|err| {
                    eprintln!("Cannot get variation coefficient: '{}'", err.to_string());
                    process::exit(1);
                })
            })
            .collect()
    });
    let minimize_routes = matches.value_of(MINIMIZE_ROUTES_ARG_NAME).unwrap().parse::<bool>().unwrap_or_else(|err| {
        eprintln!("Cannot get minimize routes: '{}'", err.to_string());
        process::exit(1);
    });
    let init_solution = matches.value_of(INIT_SOLUTION_ARG_NAME).map(|path| open_file(path, "init solution"));
    let matrix_files = matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| open_file(path, "routing matrix")).collect());
    let out_solution = matches.value_of(OUT_SOLUTION_ARG_NAME).map(|path| create_file(path, "out solution"));

    match formats.get(problem_format) {
        Some((problem_reader, init_reader, solution_writer)) => {
            match problem_reader.0(problem_file, matrix_files) {
                Ok(problem) => {
                    let problem = Arc::new(problem);
                    let solution = init_solution.and_then(|file| init_reader.0(file, problem.clone()));
                    let solution = SolverBuilder::default()
                        .with_init_solution(solution.map(|s| (problem.clone(), Arc::new(s))))
                        .with_minimize_routes(minimize_routes)
                        .with_max_generations(max_generations)
                        .with_variation_coefficient(variation_coefficient)
                        .with_max_time(max_time)
                        .build()
                        .solve(problem.clone());
                    match solution {
                        Some(solution) => {
                            let out_buffer: BufWriter<Box<dyn Write>> = if let Some(out_solution) = out_solution {
                                BufWriter::new(Box::new(out_solution))
                            } else {
                                BufWriter::new(Box::new(stdout()))
                            };
                            solution_writer.0(&problem, solution.0, out_buffer).unwrap()
                        }
                        None => println!("Cannot find any solution"),
                    };
                }
                Err(error) => {
                    eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                    process::exit(1);
                }
            };
        }
        None => {
            eprintln!("Unknown format: '{}'", problem_format);
            process::exit(1);
        }
    }
}

fn open_file(path: &str, description: &str) -> File {
    File::open(path).unwrap_or_else(|err| {
        eprintln!("Cannot open {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}

fn create_file(path: &str, description: &str) -> File {
    File::create(path).unwrap_or_else(|err| {
        eprintln!("Cannot create {} file '{}': '{}'", description, path, err.to_string());
        process::exit(1);
    })
}
