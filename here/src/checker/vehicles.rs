use crate::checker::models::SolutionInfo;
use crate::json::problem::Matrix;

pub fn check_vehicles(solution: &SolutionInfo, matrices: &[Matrix]) -> Result<(), String> {
    check_no_capacity_violation(solution)?;
    check_tour_has_proper_distances(solution, matrices)?;
    check_tour_has_proper_durations(solution, matrices)?;
    check_tour_has_proper_statistic(solution, matrices)?;

    Ok(())
}

fn check_no_capacity_violation(solution: &SolutionInfo) -> Result<(), String> {
    unimplemented!()
}

fn check_tour_has_proper_distances(solution: &SolutionInfo, matrices: &[Matrix]) -> Result<(), String> {
    unimplemented!()
}

fn check_tour_has_proper_durations(solution: &SolutionInfo, matrices: &[Matrix]) -> Result<(), String> {
    unimplemented!()
}

fn check_tour_has_proper_statistic(solution: &SolutionInfo, matrices: &[Matrix]) -> Result<(), String> {
    unimplemented!()
}
