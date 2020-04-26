use super::*;
use crate::models::common::Objective;

/// An objective function which counts total amount of unassigned jobs.
pub struct TotalUnassignedJobs {}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self {}
    }
}

impl Objective for TotalUnassignedJobs {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let fitness_a = a.solution.unassigned.len();
        let fitness_b = b.solution.unassigned.len();

        fitness_a.cmp(&fitness_b)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a.solution.unassigned.len() as f64 - b.solution.unassigned.len() as f64
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.solution.unassigned.len() as f64
    }
}
