use super::Tuple;
use crate::refinement::objectives::Objective;
use std::cmp::Ordering;

// We define three objectives
pub struct Objective1;
pub struct Objective2;
pub struct Objective3;

impl Objective for Objective1 {
    type Solution = Tuple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        a.0.cmp(&b.0)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.0 as f64) - (b.0 as f64)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.0 as f64
    }
}

impl Objective for Objective2 {
    type Solution = Tuple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        a.1.cmp(&b.1)
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.1 as f64) - (b.1 as f64)
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.1 as f64
    }
}

// Objective3 is defined on the sum of the tuple values.
impl Objective for Objective3 {
    type Solution = Tuple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        (a.0 + a.1).cmp(&(b.0 + b.1))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (a.0 + a.1) as f64 - (b.0 + b.1) as f64
    }

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        (solution.0 + solution.1) as f64
    }
}
