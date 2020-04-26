use super::*;

use crate::models::common::Objective;
use crate::utils::compare_floats;

/// An objective function which counts total amount of routes.
pub struct TotalRoutes {
    is_minimization: bool,
}

impl Default for TotalRoutes {
    fn default() -> Self {
        Self { is_minimization: true }
    }
}

impl TotalRoutes {
    pub fn new_minimized() -> Self {
        Self { is_minimization: true }
    }

    pub fn new_maximized() -> Self {
        Self { is_minimization: false }
    }
}

impl Objective for TotalRoutes {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let fitness_a = a.solution.routes.len() as f64;
        let fitness_b = b.solution.routes.len() as f64;

        let (fitness_a, fitness_b) =
            if self.is_minimization { (fitness_a, fitness_b) } else { (-1. * fitness_a, -1. * fitness_b) };

        compare_floats(fitness_a, fitness_b)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        a.solution.routes.len() as f64 - b.solution.routes.len() as f64
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.solution.routes.len() as f64
    }
}
