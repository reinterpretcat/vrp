use crate::checker::CheckerContext;
use crate::format::problem::{Matrix, PragmaticProblem, Problem};
use crate::format::solution::{create_solution, Solution};
use std::cmp::Ordering::Less;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::refinement::mutation::{Recreate, RecreateWithCheapest};
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::DefaultRandom;
use vrp_solver::SolverBuilder;

/// Runs solver with cheapest insertion heuristic.
pub fn solve_with_cheapest_insertion(problem: Problem, matrices: Option<Vec<Matrix>>) -> Solution {
    let problem_copy = problem.clone();
    let matrices_copy = matrices.clone();

    let problem = get_core_problem(problem, matrices);
    let mut refinement_ctx = RefinementContext::new(problem.clone());

    let solution = RecreateWithCheapest::default()
        .run(&mut refinement_ctx, InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())))
        .solution
        .to_solution(problem.extras.clone());

    let solution = create_solution(problem.as_ref(), &solution);

    assert_eq!(CheckerContext::new(problem_copy, matrices_copy, solution.clone()).check().err(), None);

    sort_all_data(solution)
}

/// Runs solver with default metaheuristic and default amount of generations.
pub fn solve_with_metaheuristic(problem: Problem, matrices: Option<Vec<Matrix>>) -> Solution {
    solve_with_metaheuristic_and_iterations(problem, matrices, 100)
}

/// Runs solver with default metaheuristic and specified amount of generations.
pub fn solve_with_metaheuristic_and_iterations(
    problem: Problem,
    matrices: Option<Vec<Matrix>>,
    generations: usize,
) -> Solution {
    let problem_copy = problem.clone();
    let matrices_copy = matrices.clone();

    let problem = get_core_problem(problem, matrices);

    let (solution, _, _) = SolverBuilder::default() //
        .with_max_generations(Some(generations))
        .build()
        .solve(problem.clone())
        .unwrap();

    let solution = sort_all_data(create_solution(problem.as_ref(), &solution));

    assert_eq!(CheckerContext::new(problem_copy, matrices_copy, solution.clone()).check().err(), None);

    solution
}

fn get_core_problem(problem: Problem, matrices: Option<Vec<Matrix>>) -> Arc<CoreProblem> {
    Arc::new(
        if let Some(matrices) = matrices { (problem, matrices).read_pragmatic() } else { problem.read_pragmatic() }
            .ok()
            .unwrap(),
    )
}

/// Sorts some solution properties in lexicographical order to simplify test assertions.
fn sort_all_data(solution: Solution) -> Solution {
    let mut solution = solution;

    solution.tours.sort_by(|a, b| a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less));
    solution.unassigned.sort_by(|a, b| a.job_id.partial_cmp(&b.job_id).unwrap_or(Less));

    solution
}
