//! A helper module which contains functionality to run feasibility checks on solution.

#[cfg(test)]
#[path = "../../../tests/unit/extensions/check/check_test.rs"]
mod check_test;

use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::deserialize_solution;

use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_pragmatic::format::FormatError;

/// Checks pragmatic solution feasibility.
pub fn check_pragmatic_solution<F: Read>(
    problem_reader: BufReader<F>,
    solution_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<(), Vec<String>> {
    let problem = deserialize_problem(problem_reader)
        .map_err(|errs| vec![format!("cannot read problem: '{}'", FormatError::format_many(&errs, ","))])?;

    let solution =
        deserialize_solution(solution_reader).map_err(|err| vec![format!("cannot read solution: '{}'", err)])?;

    let matrices = if let Some(matrices_readers) = matrices_readers {
        Some(
            matrices_readers
                .into_iter()
                .map(|file| {
                    deserialize_matrix(BufReader::new(file))
                        .map_err(|errs| vec![format!("cannot read matrix: '{}'", FormatError::format_many(&errs, ","))])
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        None
    };

    let core_problem = Arc::new(
        (problem.clone(), matrices.clone())
            .read_pragmatic()
            .map_err(|err| vec![format!("cannot read pragmatic problem: {}", FormatError::format_many(&err, ","))])?,
    );

    CheckerContext::new(core_problem, problem, matrices, solution).check()
}
