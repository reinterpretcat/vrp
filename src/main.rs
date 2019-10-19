#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

extern crate clap;

use clap::{App, Arg};

mod construction;
mod models;
mod refinement;
mod streams;
mod utils;

mod solver;

pub use self::solver::Solver;
use crate::models::{Problem, Solution};
use crate::streams::input::text::{LilimProblem, SolomonProblem};
use crate::streams::output::text::write_solomon_solution;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufWriter, Error};
use std::ops::Deref;
use std::process;

struct InputReader(Box<dyn Fn(File) -> Result<Problem, String>>);

struct OutputWriter(Box<dyn Fn(Solution) -> Result<(), Error>>);

fn main() {
    let readers: HashMap<&str, InputReader> = vec![
        ("solomon", InputReader(Box::new(|file: File| file.parse_solomon()))),
        ("lilim", InputReader(Box::new(|file: File| file.parse_lilim()))),
    ]
    .into_iter()
    .collect();

    let writers: HashMap<&str, OutputWriter> = vec![(
        "solomon",
        OutputWriter(Box::new(|solution: Solution| {
            write_solomon_solution(BufWriter::new(Box::new(stdout())), &solution)
        })),
    )]
    .into_iter()
    .collect();

    let matches = App::new("VRP Solver")
        .version("0.1")
        .author("Ilya Builuk <ilya.builuk@gmail.com>")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(Arg::with_name("PROBLEM").help("Sets the problem file to use").required(true).index(1))
        .arg(
            Arg::with_name("FORMAT")
                .help("Specifies the problem type")
                .required(true)
                .possible_values(readers.keys().map(|s| s.deref()).collect::<Vec<&str>>().as_slice())
                .index(2),
        )
        .get_matches();

    let problem_path = matches.value_of("PROBLEM").unwrap();
    let problem_format = matches.value_of("FORMAT").unwrap();
    let input_file = File::open(problem_path).unwrap_or_else(|err| {
        eprintln!("Cannot open file '{}': '{}'", problem_path, err.to_string());
        process::exit(1);
    });

    let solution =
        match readers.get(problem_format).ok_or_else(|| "Unknown format".to_string()).and_then(|r| r.0(input_file)) {
            Ok(problem) => Solver::default().solve(problem),
            Err(error) => {
                eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
                process::exit(1);
            }
        };

    match writers.get(problem_format) {
        Some(writer) => writer.0(solution).unwrap(),
        _ => {
            // TODO
            eprintln!("Don't know how to write solution in '{}' format", problem_format);
            process::exit(1);
        }
    }
}
