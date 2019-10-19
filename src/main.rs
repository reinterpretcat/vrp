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
use crate::models::Problem;
use crate::streams::input::text::{LilimProblem, SolomonProblem};
use std::collections::HashMap;
use std::ops::Deref;
use std::process;

struct FormatParser(Box<dyn Fn(String) -> Result<Problem, String>>);

fn main() {
    let formats: HashMap<&str, FormatParser> = vec![
        ("solomon", FormatParser(Box::new(|path: String| path.parse_solomon()))),
        ("lilim", FormatParser(Box::new(|path: String| path.parse_lilim()))),
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
    let format_parser = formats.get(problem_format).unwrap();

    match format_parser.0(problem_path.to_string()) {
        Ok(problem) => unimplemented!(),
        Err(error) => {
            eprintln!("Cannot read {} problem from '{}': '{}'", problem_format, problem_path, error);
            process::exit(1);
        }
    }
}
