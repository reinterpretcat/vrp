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

use clap::{App, Arg};

use crate::models::{Problem, Solution};
use crate::streams::input::text::{LilimProblem, SolomonProblem};
use crate::streams::output::text::{write_lilim_solution, write_solomon_solution};

pub use self::solver::Solver;

mod construction;
mod models;
mod refinement;
mod streams;
mod utils;

mod solver;

struct InputReader(Box<dyn Fn(File) -> Result<Problem, String>>);

struct OutputWriter(Box<dyn Fn(Solution) -> Result<(), Error>>);

fn main() {
    let formats: HashMap<&str, (InputReader, OutputWriter)> = vec![
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
                .possible_values(formats.keys().map(|s| s.deref()).collect::<Vec<&str>>().as_slice())
                .index(2),
        )
        .get_matches();

    let problem_path = matches.value_of("PROBLEM").unwrap();
    let problem_format = matches.value_of("FORMAT").unwrap();
    let input_file = File::open(problem_path).unwrap_or_else(|err| {
        eprintln!("Cannot open file '{}': '{}'", problem_path, err.to_string());
        process::exit(1);
    });

    match formats.get(problem_format) {
        Some((reader, writer)) => {
            let solution = match reader.0(input_file) {
                Ok(problem) => Solver::default().solve(problem),
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
