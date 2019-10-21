#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

extern crate clap;

use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufWriter, Error};
use std::ops::Deref;
use std::process;

use clap::{App, Arg, ArgMatches};

use crate::models::{Problem, Solution};
use crate::streams::input::text::{LilimProblem, SolomonProblem};
use crate::streams::output::text::{write_lilim_solution, write_solomon_solution};

pub use self::solver::Solver;
use crate::solver::SolverBuilder;

mod construction;
mod models;
mod refinement;
mod streams;
mod utils;

mod solver;

struct InputReader(Box<dyn Fn(File) -> Result<Problem, String>>);

struct OutputWriter(Box<dyn Fn(Solution) -> Result<(), Error>>);

fn get_formats<'a>() -> HashMap<&'a str, (InputReader, OutputWriter)> {
    vec![
        (
            "solomon",
            (
                InputReader(Box::new(|file: File| file.parse_solomon())),
                OutputWriter(Box::new(|solution: Solution| {
                    write_solomon_solution(BufWriter::new(Box::new(stdout())), &solution)
                })),
            ),
        ),
        (
            "lilim",
            (
                InputReader(Box::new(|file: File| file.parse_lilim())),
                OutputWriter(Box::new(|solution: Solution| {
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
            Arg::with_name("minimize-routes")
                .help("Prefer less routes over total cost")
                .short("r")
                .long("minimize-routes")
                .required(false)
                .default_value("true")
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
        eprintln!("Cannot open file '{}': '{}'", problem_path, err.to_string());
        process::exit(1);
    });

    // optional
    let max_generations = matches.value_of("max-generations").unwrap().parse::<usize>().unwrap_or_else(|err| {
        eprintln!("Cannot get max-generations: '{}'", err.to_string());
        process::exit(1);
    });
    let minimize_routes = matches.value_of("minimize-routes").unwrap().parse::<bool>().unwrap_or_else(|err| {
        eprintln!("Cannot get minimize-routes: '{}'", err.to_string());
        process::exit(1);
    });

    match formats.get(problem_format) {
        Some((reader, writer)) => {
            let solution = match reader.0(input_file) {
                Ok(problem) => SolverBuilder::new()
                    .with_minimize_routes(minimize_routes)
                    .with_max_generations(max_generations)
                    .build()
                    .solve(problem),
                Err(error) => {
                    eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                    process::exit(1);
                }
            };

            match solution {
                Some(solution) => writer.0(solution.0).unwrap(),
                None => println!("Cannot find any solution"),
            };
        }
        None => {
            eprintln!("Unknown format: '{}'", problem_format);
            process::exit(1);
        }
    }
}
