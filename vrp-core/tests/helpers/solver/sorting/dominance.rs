use crate::helpers::solver::sorting::Tuple;
use crate::models::Objective;
use std::cmp::Ordering;

pub struct TupleObjective;

impl Objective for TupleObjective {
    type Solution = Tuple;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        if a.0 < b.0 && a.1 <= b.1 {
            Ordering::Less
        } else if a.0 <= b.0 && a.1 < b.1 {
            Ordering::Less
        } else if a.0 > b.0 && a.1 >= b.1 {
            Ordering::Greater
        } else if a.0 >= b.0 && a.1 > b.1 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unimplemented!()
    }

    fn fitness(&self, _solution: &Self::Solution) -> f64 {
        unimplemented!()
    }
}
