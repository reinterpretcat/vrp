use super::*;
use vrp_pragmatic::format::problem::Problem;

/// Generates meaningful problem from the prototype.
/// There is another problem generation implementation in `vrp-pragmatic` crate, used by tests.
/// Its main goal is to discover problem space by generating many, potentially unrealistic, problems
/// using property based approach. This implementation, in contrast, focuses on generating realistic
/// problems.
pub fn generate_from_prototype(problem: &Problem, job_size: usize, area_size: Option<f64>) -> Result<Problem, String> {
    if problem.plan.jobs.len() < 3 {
        return Err("at least three jobs should be defined".to_string());
    }

    Ok(Problem {
        plan: generate_plan(&problem, job_size, area_size)?,
        fleet: problem.fleet.clone(),
        objectives: problem.objectives.clone(),
        config: problem.config.clone(),
    })
}
