use crate::json::problem::{Matrix, PragmaticProblem, Problem};
use crate::json::solution::{create_solution, Solution};
use std::cmp::Ordering::Less;
use std::sync::Arc;
use vrp_core::construction::states::InsertionContext;
use vrp_core::refinement::mutation::{Recreate, RecreateWithCheapest};
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::DefaultRandom;
use vrp_solver::SolverBuilder;

pub fn solve_with_heuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    let problem = Arc::new((problem, matrices).read_pragmatic().unwrap());
    let mut refinement_ctx = RefinementContext::new(problem.clone());
    let solution = RecreateWithCheapest::default()
        .run(&mut refinement_ctx, InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())))
        .solution
        .to_solution(problem.extras.clone());
    sort_all_data(create_solution(problem.as_ref(), &solution))
}

pub fn solve_with_metaheuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    solve_with_metaheuristic_and_iterations(problem, matrices, 100)
}

pub fn solve_with_metaheuristic_and_iterations(problem: Problem, matrices: Vec<Matrix>, iterations: usize) -> Solution {
    let problem = Arc::new((problem, matrices).read_pragmatic().unwrap());
    let solution =
        SolverBuilder::default().with_max_generations(Some(iterations)).build().solve(problem.clone()).unwrap().0;
    sort_all_data(create_solution(problem.as_ref(), &solution))
}

fn sort_all_data(solution: Solution) -> Solution {
    let mut solution = solution;

    solution.tours.sort_by(|a, b| a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less));
    solution.unassigned.sort_by(|a, b| a.job_id.partial_cmp(&b.job_id).unwrap_or(Less));

    solution
}
