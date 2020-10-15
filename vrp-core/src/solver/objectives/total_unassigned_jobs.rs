use super::*;
use crate::algorithms::nsga2::Objective;
use crate::models::problem::Job;
use crate::utils::compare_floats;
use std::ops::Deref;
use std::sync::Arc;

/// A type which allows to control how job is estimated in objective fitness
pub type UnassignedJobEstimator = Arc<dyn Fn(&InsertionContext, &Job, i32) -> f64 + Send + Sync>;

/// An objective function which minimizes amount of unassigned jobs as a target.
pub struct TotalUnassignedJobs {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl TotalUnassignedJobs {
    /// Creates a new instance of `TotalUnassignedJobs`.
    pub fn new(unassigned_job_estimator: UnassignedJobEstimator) -> Self {
        Self { unassigned_job_estimator }
    }
}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self::new(Arc::new(|_, _, _| 1.))
    }
}

impl Objective for TotalUnassignedJobs {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        compare_floats(self.fitness(a), self.fitness(b))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        self.fitness(a) - self.fitness(b)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .unassigned
            .iter()
            .map(|(job, code)| self.unassigned_job_estimator.deref()(solution, job, *code))
            .sum::<f64>()
    }
}
