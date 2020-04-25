use super::*;

/// An objective function which counts total amount of unassigned jobs.
pub struct TotalUnassignedJobs {
    unassigned_goal: Option<(f64, bool)>,
    variation_goal: Option<VariationCoefficient>,
}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self { unassigned_goal: None, variation_goal: None }
    }
}

impl TotalUnassignedJobs {
    pub fn new(desired_unassigned: Option<usize>, variation_goal: Option<(usize, f64)>) -> Self {
        Self {
            unassigned_goal: desired_unassigned.map(|unassigned| (unassigned as f64, true)),
            variation_goal: variation_goal
                .map(|(sample, threshold)| VariationCoefficient::new(sample, threshold, "unassigned_vc")),
        }
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
