#[cfg(test)]
#[path = "../../../tests/unit/solver/objectives/total_unassigned_jobs_test.rs"]
mod total_unassigned_jobs_test;

use super::*;
use crate::construction::heuristics::UnassignmentInfo;
use crate::models::problem::Job;
use rosomaxa::prelude::*;
use std::ops::Deref;
use std::sync::Arc;

/// A type which allows to control how job is estimated in objective fitness
pub type UnassignedJobEstimator = Arc<dyn Fn(&InsertionContext, &Job, &UnassignmentInfo) -> f64 + Send + Sync>;

/// An objective function which minimizes amount of unassigned jobs as a target.
pub struct TotalUnassignedJobs {
    unassigned_job_estimator: UnassignedJobEstimator,
}

impl TotalUnassignedJobs {
    /// Creates a new instance of `TotalUnassignedJobs`.
    pub fn new(unassigned_job_estimator: UnassignedJobEstimator) -> Self {
        Self { unassigned_job_estimator }
    }

    /// Checks the edge case when at least one solution has no routes and amount of unassigned is
    /// equal to another solution (can happen with conditional jobs).
    fn is_edge_case(
        &self,
        a: &<TotalUnassignedJobs as Objective>::Solution,
        b: &<TotalUnassignedJobs as Objective>::Solution,
        fitness_a: f64,
        fitness_b: f64,
    ) -> bool {
        let with_empty_routes = a.solution.routes.is_empty() || b.solution.routes.is_empty();
        let with_same_fitness = compare_floats(fitness_a, fitness_b) == Ordering::Equal;

        with_same_fitness && with_empty_routes
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
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        let order = compare_floats(fitness_a, fitness_b);

        match (self.is_edge_case(a, b, fitness_a, fitness_b), order) {
            (true, _) => b.solution.routes.len().cmp(&a.solution.routes.len()),
            _ => order,
        }
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .unassigned
            .iter()
            .map(|(job, code)| self.unassigned_job_estimator.deref()(solution, job, code))
            .sum::<f64>()
    }
}
