//! Provides the way to control job assignment.

use super::*;
use std::ops::Deref;

/// A type which allows to control how job is estimated in objective fitness.
pub type UnassignedJobEstimator = Arc<dyn Fn(&SolutionContext, &Job) -> f64 + Send + Sync>;

/// Creates a feature to minimize amount of unassigned jobs.
pub fn minimize_unassigned_jobs(unassigned_job_estimator: UnassignedJobEstimator) -> Result<Feature, String> {
    FeatureBuilder::default().with_objective(Arc::new(MinimizeUnassignedObjective { unassigned_job_estimator })).build()
}

/// Maximizes a total value of served jobs.
pub fn maximize_job_value() -> Result<Feature, String> {
    unimplemented!()
}

struct MinimizeUnassignedObjective {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl Objective for MinimizeUnassignedObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .unassigned
            .iter()
            .map(|(job, _)| self.unassigned_job_estimator.deref()(&solution.solution, job))
            .sum::<f64>()
    }
}

impl FeatureObjective for MinimizeUnassignedObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, job, .. } => {
                -1. * self.unassigned_job_estimator.deref()(solution_ctx, job)
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}
