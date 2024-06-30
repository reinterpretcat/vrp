//! Provides the way to control job assignment.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/minimize_unassigned_test.rs"]
mod minimize_unassigned_test;

use super::*;

/// Provides a way to build a feature to minimize amount of unassigned jobs.
pub struct MinimizeUnassignedBuilder {
    name: String,
    job_estimator: Option<UnassignedJobEstimator>,
}

impl MinimizeUnassignedBuilder {
    /// Creates a new instance of `MinimizeUnassignedBuilder`
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), job_estimator: None }
    }

    /// Sets a job estimator function which responsible for cost estimate of unassigned jobs.
    /// Optional. Default is the implementation which gives 1 as estimate to any unassisgned job.
    pub fn set_job_estimator<F>(mut self, func: F) -> Self
    where
        F: Fn(&SolutionContext, &Job) -> f64 + Send + Sync + 'static,
    {
        self.job_estimator = Some(Arc::new(func));
        self
    }

    /// Builds a feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let unassigned_job_estimator = self.job_estimator.take().unwrap_or_else(|| Arc::new(|_, _| 1.));

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_objective(MinimizeUnassignedObjective { unassigned_job_estimator })
            .build()
    }
}

/// A type which allows to control how job is estimated in objective fitness.
type UnassignedJobEstimator = Arc<dyn Fn(&SolutionContext, &Job) -> f64 + Send + Sync>;

struct MinimizeUnassignedObjective {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl FeatureObjective for MinimizeUnassignedObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .unassigned
            .keys()
            .map(|job| (self.unassigned_job_estimator)(&solution.solution, job))
            .sum::<f64>()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, job, .. } => -1. * (self.unassigned_job_estimator)(solution_ctx, job),
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}
