use core::construction::states::InsertionContext;
use core::models::{Problem, Solution};
use core::refinement::recreate::{Recreate, RecreateWithCheapest};
use core::utils::DefaultRandom;
use std::sync::Arc;

pub fn solve_with_cheapest(problem: Arc<Problem>) -> Solution {
    RecreateWithCheapest::default()
        .run(InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::new())))
        .solution
        .to_solution(problem.extras.clone())
}
