use super::*;
use crate::helpers::refinement::population::*;

#[test]
fn can_compare_non_dominant_relations() {
    let a = &Tuple(1, 2);
    let b = &Tuple(2, 1);

    // Non-domination due to reflexivity
    assert_eq!(Ordering::Equal, TupleDominanceOrd.dominance_ord(a, a));
    assert_eq!(Ordering::Equal, TupleDominanceOrd.dominance_ord(b, b));

    // Non-domination
    assert_eq!(Ordering::Equal, TupleDominanceOrd.dominance_ord(a, b));
    assert_eq!(Ordering::Equal, TupleDominanceOrd.dominance_ord(b, a));
}

#[test]
fn can_compare_dominant_relations() {
    let a = &Tuple(1, 2);
    let b = &Tuple(1, 3);
    let c = &Tuple(0, 2);

    // a < b
    assert_eq!(Ordering::Less, TupleDominanceOrd.dominance_ord(a, b));
    // c < a
    assert_eq!(Ordering::Less, TupleDominanceOrd.dominance_ord(c, a));
    // transitivity => c < b
    assert_eq!(Ordering::Less, TupleDominanceOrd.dominance_ord(c, b));

    // Just reverse the relation: for all a, b: a < b => b > a

    // b > a
    assert_eq!(Ordering::Greater, TupleDominanceOrd.dominance_ord(b, a));
    // a > c
    assert_eq!(Ordering::Greater, TupleDominanceOrd.dominance_ord(a, c));
    // transitivity => b > c
    assert_eq!(Ordering::Greater, TupleDominanceOrd.dominance_ord(b, c));
}
