use super::*;
use crate::helpers::algorithms::nsga2::*;

/// Creates `n_fronts` with each having `n` solutions in it.
pub fn create_solutions_with_n_fronts(n: usize, n_fronts: usize) -> (Vec<Pair>, Vec<Vec<usize>>) {
    let mut solutions = Vec::with_capacity(n * n_fronts);
    let mut expected_fronts = Vec::with_capacity(n_fronts);

    for front in 0..n_fronts {
        let mut current_front = Vec::with_capacity(n);
        for i in 0..n {
            solutions.push(Pair(front + i, front + n - i));
            current_front.push(front * n + i);
        }
        expected_fronts.push(current_front);
    }

    return (solutions, expected_fronts);
}

fn get_solutions() -> Vec<Pair> {
    vec![Pair(1, 2), Pair(1, 2), Pair(2, 1), Pair(1, 3), Pair(0, 2)]
}

#[test]
fn can_compare_dominant_relations() {
    let objective = PairMultiObjective::new(vec![]);
    let a = &Pair(1, 2);
    let b = &Pair(1, 3);
    let c = &Pair(0, 2);

    // a < b
    assert_eq!(Ordering::Less, objective.total_order(a, b));
    // c < a
    assert_eq!(Ordering::Less, objective.total_order(c, a));
    // transitivity => c < b
    assert_eq!(Ordering::Less, objective.total_order(c, b));

    // Just reverse the relation: for all a, b: a < b => b > a

    // b > a
    assert_eq!(Ordering::Greater, objective.total_order(b, a));
    // a > c
    assert_eq!(Ordering::Greater, objective.total_order(a, c));
    // transitivity => b > c
    assert_eq!(Ordering::Greater, objective.total_order(b, c));
}

#[test]
fn can_use_simple_objectives() {
    let a = &Pair(1, 2);
    let b = &Pair(2, 1);
    assert_eq!(Ordering::Less, PairObjective1.total_order(a, b));
    assert_eq!(Ordering::Greater, PairObjective2.total_order(a, b));
    assert_eq!(Ordering::Equal, PairObjective3.total_order(a, b));

    assert_eq!(-1.0, PairObjective1.distance(a, b));
    assert_eq!(1.0, PairObjective2.distance(a, b));
    assert_eq!(0.0, PairObjective3.distance(a, b));
}

#[test]
fn test_non_dominated_sort() {
    let objective = PairMultiObjective::new(vec![]);
    let solutions = get_solutions();

    let f0 = non_dominated_sort(&solutions, &objective);
    assert_eq!(0, f0.rank());
    assert_eq!(&[2, 4], f0.current_front.as_slice());

    let f1 = f0.next_front();
    assert_eq!(1, f1.rank());
    assert_eq!(&[0, 1], f1.current_front.as_slice());

    let f2 = f1.next_front();
    assert_eq!(2, f2.rank());
    assert_eq!(&[3], f2.current_front.as_slice());

    let f3 = f2.next_front();
    assert_eq!(3, f3.rank());
    assert_eq!(true, f3.is_empty());
}

fn test_fronts(n: usize, n_fronts: usize) {
    let objective = PairMultiObjective::new(vec![]);
    let (solutions, expected_fronts) = create_solutions_with_n_fronts(n, n_fronts);

    let mut f = non_dominated_sort(&solutions, &objective);
    for (expected_rank, expected_front) in expected_fronts.iter().enumerate() {
        assert_eq!(expected_rank, f.rank());
        assert_eq!(&expected_front[..], f.current_front.as_slice());
        f = f.next_front();
    }
    assert_eq!(true, f.is_empty());
}

#[test]
fn test_non_dominated_sort_5_5() {
    test_fronts(5, 5);
}

#[test]
fn test_non_dominated_sort_1000_5() {
    test_fronts(1_000, 5);
}
