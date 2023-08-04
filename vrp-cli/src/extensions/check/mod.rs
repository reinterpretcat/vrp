//! A helper module which contains functionality to run feasibility checks on solution.

#[cfg(test)]
#[path = "../../../tests/unit/extensions/check/check_test.rs"]
mod check_test;

use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::prelude::GenericError;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::solution::deserialize_solution;

/// Checks pragmatic solution feasibility.
pub fn check_pragmatic_solution<F: Read>(
    problem_reader: BufReader<F>,
    solution_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<(), Vec<GenericError>> {
    let problem =
        deserialize_problem(problem_reader).map_err(|errs| vec![format!("cannot read problem: '{errs}'").into()])?;

    let solution =
        deserialize_solution(solution_reader).map_err(|err| vec![format!("cannot read solution: '{err}'").into()])?;

    let matrices = if let Some(matrices_readers) = matrices_readers {
        Some(
            matrices_readers
                .into_iter()
                .map(|file| {
                    deserialize_matrix(BufReader::new(file))
                        .map_err(|errs| vec![format!("cannot read matrix: '{errs}'").into()])
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        None
    };

    let core_problem = Arc::new(
        (problem.clone(), matrices.clone())
            .read_pragmatic()
            .map_err(|errs| vec![format!("cannot read pragmatic problem: '{errs}'").into()])?,
    );

    CheckerContext::new(core_problem, problem, matrices, solution).and_then(|ctx| ctx.check())
}
