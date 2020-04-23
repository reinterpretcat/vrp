use crate::helpers::refinement::population::Tuple;
use crate::refinement::population::DominanceOrd;
use std::cmp::Ordering;

pub struct TupleDominanceOrd;

impl DominanceOrd for TupleDominanceOrd {
    type T = Tuple;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
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
}
