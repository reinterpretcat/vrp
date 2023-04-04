//! Provides the way to control job assignment.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/minimize_unassigned_test.rs"]
mod minimize_unassigned_test;

use super::*;
use std::cmp::Ordering;

/// A type which allows to control how job is estimated in objective fitness.
pub type UnassignedJobEstimator = Arc<dyn Fn(&SolutionContext, &Job) -> f64 + Send + Sync>;

/// Creates a feature to minimize amount of unassigned jobs.
pub fn create_minimize_unassigned_jobs_feature(
    name: &str,
    unassigned_job_estimator: UnassignedJobEstimator,
) -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(MinimizeUnassignedObjective { unassigned_job_estimator })
        .build()
}

struct MinimizeUnassignedObjective {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl Objective for MinimizeUnassignedObjective {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        let order = compare_floats(fitness_a, fitness_b);

        match (is_edge_case(a, b, fitness_a, fitness_b), order) {
            (true, _) => b.solution.routes.len().cmp(&a.solution.routes.len()),
            _ => order,
        }
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .unassigned
            .iter()
            .map(|(job, _)| (self.unassigned_job_estimator)(&solution.solution, job))
            .sum::<f64>()
    }
}

impl FeatureObjective for MinimizeUnassignedObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, job, .. } => -1. * (self.unassigned_job_estimator)(solution_ctx, job),
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

/// Checks the edge case when at least one solution has no routes and amount of unassigned is
/// equal to another solution (can happen with conditional jobs).
fn is_edge_case(a: &InsertionContext, b: &InsertionContext, fitness_a: f64, fitness_b: f64) -> bool {
    let with_empty_routes = a.solution.routes.is_empty() || b.solution.routes.is_empty();
    let with_same_fitness = compare_floats(fitness_a, fitness_b) == Ordering::Equal;

    with_same_fitness && with_empty_routes
}
