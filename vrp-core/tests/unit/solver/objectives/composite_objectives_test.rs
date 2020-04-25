use crate::helpers::solver::sorting::*;
use crate::models::Objective;
use crate::solver::objectives::*;
use std::cmp::Ordering;

#[test]
fn can_compare_non_dominant_relations() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);

    // Non-domination due to reflexivity
    assert_eq!(Ordering::Equal, TupleObjective.total_order(a, a));
    assert_eq!(Ordering::Equal, TupleObjective.total_order(b, b));

    // Non-domination
    assert_eq!(Ordering::Equal, TupleObjective.total_order(a, b));
    assert_eq!(Ordering::Equal, TupleObjective.total_order(b, a));
}

#[test]
fn can_compare_dominant_relations() {
    let a = &Tuple(1, 2);
    let b = &Tuple(1, 3);
    let c = &Tuple(0, 2);

    // a < b
    assert_eq!(Ordering::Less, TupleObjective.total_order(a, b));
    // c < a
    assert_eq!(Ordering::Less, TupleObjective.total_order(c, a));
    // transitivity => c < b
    assert_eq!(Ordering::Less, TupleObjective.total_order(c, b));

    // Just reverse the relation: for all a, b: a < b => b > a

    // b > a
    assert_eq!(Ordering::Greater, TupleObjective.total_order(b, a));
    // a > c
    assert_eq!(Ordering::Greater, TupleObjective.total_order(a, c));
    // transitivity => b > c
    assert_eq!(Ordering::Greater, TupleObjective.total_order(b, c));
}

#[test]
fn can_use_simple_objectives() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);
    assert_eq!(Ordering::Less, Objective1.total_order(a, b));
    assert_eq!(Ordering::Greater, Objective2.total_order(a, b));
    assert_eq!(Ordering::Equal, Objective3.total_order(a, b));

    assert_eq!(-1.0, Objective1.distance(a, b));
    assert_eq!(1.0, Objective2.distance(a, b));
    assert_eq!(0.0, Objective3.distance(a, b));
}

#[test]
fn can_use_hierarchy_objectives() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);
    let c = &Tuple(1, 1);

    let hierarchy = HierarchyObjective::new(
        MultiObjective::new(vec![Box::new(Objective1)]),
        MultiObjective::new(vec![Box::new(Objective2)]),
    );

    assert_eq!(Ordering::Less, hierarchy.total_order(a, b));
    assert_eq!(Ordering::Greater, hierarchy.total_order(b, a));

    assert_eq!(Ordering::Greater, hierarchy.total_order(a, c));
    assert_eq!(Ordering::Less, hierarchy.total_order(c, a));
}
