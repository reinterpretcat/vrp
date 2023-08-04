#[cfg(test)]
#[path = "../../tests/unit/commands/check_test.rs"]
mod check_test;

use super::*;
use vrp_core::prelude::GenericError;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "problem-file";
const SOLUTION_ARG_NAME: &str = "solution-file";
const MATRIX_ARG_NAME: &str = "matrix";

pub fn get_check_app() -> Command {
    Command::new("check")
        .about("Provides the way to check solution feasibility")
        .arg(Arg::new(FORMAT_ARG_NAME).help("Specifies input type").required(true).value_parser(["pragmatic"]).index(1))
        .arg(
            Arg::new(PROBLEM_ARG_NAME)
                .help("Sets input files which contain a VRP definition")
                .short('p')
                .long(PROBLEM_ARG_NAME)
                .required(true)
                .num_args(1..),
        )
        .arg(Arg::new(SOLUTION_ARG_NAME).help("Sets solution file").short('s').long(SOLUTION_ARG_NAME).required(true))
        .arg(
            Arg::new(MATRIX_ARG_NAME)
                .help("Specifies path to file with routing matrix")
                .short('m')
                .long(MATRIX_ARG_NAME)
                .required(false)
                .num_args(1..),
        )
}

pub fn run_check(matches: &ArgMatches) -> Result<(), GenericError> {
    let input_format = matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();
    check_solution(matches, input_format, PROBLEM_ARG_NAME, SOLUTION_ARG_NAME, MATRIX_ARG_NAME)
}
