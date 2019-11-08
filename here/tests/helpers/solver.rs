use crate::json::problem::{HereProblem, Matrix, Problem};
use crate::json::solution::{create_solution, Solution};
use core::construction::states::InsertionContext;
use core::refinement::recreate::{Recreate, RecreateWithCheapest};
use core::solver::SolverBuilder;
use core::utils::DefaultRandom;
use std::sync::Arc;

pub fn solve_with_heuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    let problem = Arc::new((problem, matrices).read_here().unwrap());
    let solution = RecreateWithCheapest::default()
        .run(InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new())))
        .solution
        .to_solution(problem.extras.clone());
    create_solution(problem.as_ref(), &solution)
}

pub fn solve_with_metaheuristic(problem: Problem, matrices: Vec<Matrix>) -> Solution {
    let problem = Arc::new((problem, matrices).read_here().unwrap());
    let solution = SolverBuilder::new().with_max_generations(10).build().solve(problem.clone()).unwrap().0;
    create_solution(problem.as_ref(), &solution)
}
