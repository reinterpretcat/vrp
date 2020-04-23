use crate::helpers::refinement::population::*;
use crate::refinement::population::{DominanceOrd, Objective};
use std::cmp::Ordering;

#[test]
fn can_use_simple_objectives() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);
    assert_eq!(Ordering::Less, Objective1.dominance_ord(a, b));
    assert_eq!(Ordering::Greater, Objective2.dominance_ord(a, b));
    assert_eq!(Ordering::Equal, Objective3.dominance_ord(a, b));

    assert_eq!(-1.0, Objective1.distance(a, b));
    assert_eq!(1.0, Objective2.distance(a, b));
    assert_eq!(0.0, Objective3.distance(a, b));
}

#[test]
fn can_use_hierarchy_objectives() {
    // TODO
}
