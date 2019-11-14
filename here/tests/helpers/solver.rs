use crate::json::problem::{HereProblem, Matrix, Problem};
use crate::json::solution::{create_solution, Solution};
use core::construction::states::InsertionContext;
use core::refinement::recreate::{Recreate, RecreateWithCheapest};
use core::solver::SolverBuilder;
use core::utils::DefaultRandom;
use std::cmp::Ordering::Less;
use std::sync::Arc;

pub fn solve_with_heuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    let problem = Arc::new((problem, matrices).read_here().unwrap());
    let solution = RecreateWithCheapest::default()
        .run(InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())))
        .solution
        .to_solution(problem.extras.clone());
    sort_all_data(create_solution(problem.as_ref(), &solution))
}

pub fn solve_with_metaheuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    let problem = Arc::new((problem, matrices).read_here().unwrap());
    let solution = SolverBuilder::default().with_max_generations(10).build().solve(problem.clone()).unwrap().0;
    sort_all_data(create_solution(problem.as_ref(), &solution))
}

fn sort_all_data(solution: Solution) -> Solution {
    let mut solution = solution;

    solution.tours.sort_by(|a, b| a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less));
    solution.unassigned.sort_by(|a, b| a.job_id.partial_cmp(&b.job_id).unwrap_or(Less));

    solution
}
