//! A helper module which contains functionality to run feasibility checks on solution.

use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem};
use vrp_pragmatic::format::solution::deserialize_solution;

use std::io::{BufReader, Read};
use std::process;
use vrp_pragmatic::format::FormatError;

/// Checks pragmatic solution feasibility.
pub fn check_pragmatic_solution<F: Read>(
    problem_reader: BufReader<F>,
    solution_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<(), String> {
    let problem = deserialize_problem(problem_reader).unwrap_or_else(|errs| {
        eprintln!("cannot read problem: '{}'", FormatError::format_many(&errs, ","));
        process::exit(1);
    });

    let solution = deserialize_solution(solution_reader).unwrap_or_else(|err| {
        eprintln!("cannot read solution: '{}'", err);
        process::exit(1);
    });

    let matrices = matrices_readers.map(|matrices| {
        matrices
            .into_iter()
            .map(|file| {
                deserialize_matrix(BufReader::new(file)).unwrap_or_else(|errs| {
                    eprintln!("cannot read matrix: '{}'", FormatError::format_many(&errs, ","));
                    process::exit(1);
                })
            })
            .collect::<Vec<_>>()
    });

    CheckerContext::new(problem, matrices, solution).check()
}
