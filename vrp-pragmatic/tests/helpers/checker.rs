use crate::checker::CheckerContext;
use crate::format::problem::*;
use crate::helpers::solve_with_metaheuristic_and_iterations;

/// Creates checker context solving problem the problem with defaults.
pub fn create_checker_context(problem: Problem, matrices: Option<Vec<Matrix>>) -> CheckerContext {
    let solution = solve_with_metaheuristic_and_iterations(problem.clone(), matrices.clone(), 10);

    CheckerContext::new(problem, matrices, solution)
}

/// Solves problem and checks results.
pub fn solve_and_check(problem: Problem, matrices: Option<Vec<Matrix>>) -> Result<(), String> {
    create_checker_context(problem, matrices).check()
}
