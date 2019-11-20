use crate::checker::index::create_solution_info;
use crate::checker::jobs::check_jobs;
use crate::checker::relations::check_relations;
use crate::checker::vehicles::check_vehicles;
use crate::json::problem::{Matrix, Problem};
use crate::json::solution::Solution;

mod index;
mod jobs;
mod models;
mod relations;
mod vehicles;

/**
    Validates solution correctness of specific problem. Returns first found error
    in case of incorrect solution.
*/
pub fn validate(problem: &Problem, solution: &Solution, matrices: &[Matrix]) -> Result<(), String> {
    let solution_info = create_solution_info(problem, solution)?;

    check_jobs(&solution_info)?;
    check_vehicles(&solution_info, matrices)?;
    check_relations(&solution_info)?;

    Ok(())
}
