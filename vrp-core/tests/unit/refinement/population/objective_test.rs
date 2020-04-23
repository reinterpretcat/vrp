use crate::helpers::refinement::population::*;
use crate::refinement::population::Objective;
use std::cmp::Ordering;

#[test]
fn test_objectives() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);
    assert_eq!(Ordering::Less, Objective1.total_order(a, b));
    assert_eq!(Ordering::Greater, Objective2.total_order(a, b));
    assert_eq!(Ordering::Equal, Objective3.total_order(a, b));

    assert_eq!(-1.0, Objective1.distance(a, b));
    assert_eq!(1.0, Objective2.distance(a, b));
    assert_eq!(0.0, Objective3.distance(a, b));
}
