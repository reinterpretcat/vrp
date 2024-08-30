#[cfg(test)]
#[path = "../../../tests/unit/extensions/generate/prototype_test.rs"]
mod prototype_test;

use super::*;
use vrp_core::prelude::Float;
use vrp_pragmatic::format::problem::Problem;
use vrp_pragmatic::format::Location;

/// Generates meaningful problem from the prototype.
/// There is another problem generation implementation in `vrp-pragmatic` crate, used by tests.
/// Its main goal is to discover problem space by generating many, potentially unrealistic, problems
/// using property based approach. This implementation, in contrast, focuses on generating realistic
/// problems.
pub(crate) fn generate_from_prototype(
    problem: &Problem,
    locations: Option<Vec<Location>>,
    jobs_size: usize,
    vehicle_types_size: usize,
    area_size: Option<Float>,
) -> Result<Problem, GenericError> {
    if problem.plan.jobs.len() < 3 {
        return Err("at least three jobs should be defined".into());
    }

    Ok(Problem {
        plan: generate_plan(problem, locations, jobs_size, area_size)?,
        fleet: generate_fleet(problem, vehicle_types_size),
        objectives: problem.objectives.clone(),
    })
}
