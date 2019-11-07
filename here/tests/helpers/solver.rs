use crate::json::solution::{create_solution, Solution};
use core::construction::states::InsertionContext;
use core::models::Problem;
use core::refinement::recreate::{Recreate, RecreateWithCheapest};
use core::solver::Solver;
use core::utils::DefaultRandom;
use std::sync::Arc;

pub fn solve_with_heuristic(problem: Arc<Problem>) -> Solution {
    let solution = RecreateWithCheapest::default()
        .run(InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new())))
        .solution
        .to_solution(problem.extras.clone());
    create_solution(problem.as_ref(), &solution)
}

pub fn solve_with_metaheuristic(problem: Arc<Problem>) -> Solution {
    let solution = Solver::default().solve(problem.clone()).unwrap().0;
    create_solution(problem.as_ref(), &solution)
}
