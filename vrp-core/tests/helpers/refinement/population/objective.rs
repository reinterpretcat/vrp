use super::Tuple;
use crate::refinement::population::{DominanceOrd, Objective};
use std::cmp::Ordering;

// We define three objectives
pub struct Objective1;
pub struct Objective2;
pub struct Objective3;

impl Objective for Objective1 {
    type Distance = f64;

    fn distance(&self, a: &Self::T, b: &Self::T) -> Self::Distance {
        (a.0 as f64) - (b.0 as f64)
    }
}

impl DominanceOrd for Objective1 {
    type T = Tuple;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
        a.0.cmp(&b.0)
    }
}

impl Objective for Objective2 {
    type Distance = f64;

    fn distance(&self, a: &Self::T, b: &Self::T) -> Self::Distance {
        (a.1 as f64) - (b.1 as f64)
    }
}

impl DominanceOrd for Objective2 {
    type T = Tuple;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
        a.1.cmp(&b.1)
    }
}

// Objective3 is defined on the sum of the tuple values.
impl Objective for Objective3 {
    type Distance = f64;

    fn distance(&self, a: &Self::T, b: &Self::T) -> Self::Distance {
        (a.0 + a.1) as f64 - (b.0 + b.1) as f64
    }
}

impl DominanceOrd for Objective3 {
    type T = Tuple;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
        (a.0 + a.1).cmp(&(b.0 + b.1))
    }
}
