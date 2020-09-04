use super::*;
use crate::algorithms::nsga2::non_dominated_sort::non_dominated_sort;
use crate::helpers::algorithms::nsga2::*;

#[test]
fn can_get_crowding_distance() {
    let mo = SliceMultiObjective::new(vec![
        Box::new(SliceDimensionObjective::new(0)),
        Box::new(SliceDimensionObjective::new(1)),
    ]);

    let a = vec![1., 3.];
    let b = vec![3., 1.];
    let c = vec![3., 3.];
    let d = vec![2., 2.];

    let solutions = vec![a.clone(), b.clone(), c.clone(), d.clone()];

    let f0 = non_dominated_sort(&solutions, &mo);

    let solutions = f0.iter().collect::<Vec<_>>();
    assert_eq!(3, solutions.len());
    assert_eq!(&a, solutions[0].0);
    assert_eq!(&b, solutions[1].0);
    assert_eq!(&d, solutions[2].0);

    let (crowding, stat) = assign_crowding_distance(&f0, &mo);

    assert_eq!(2, stat.len());
    assert_eq!(2.0, stat[0].spread);
    assert_eq!(2.0, stat[1].spread);

    // Same number as solutions in front
    assert_eq!(3, crowding.len());
    // All have rank 0
    assert_eq!(0, crowding[0].rank);
    assert_eq!(0, crowding[1].rank);
    assert_eq!(0, crowding[2].rank);

    let ca = crowding.iter().find(|i| i.solution.eq(&a)).unwrap();
    let cb = crowding.iter().find(|i| i.solution.eq(&b)).unwrap();
    let cd = crowding.iter().find(|i| i.solution.eq(&d)).unwrap();

    assert_eq!(INFINITY, ca.crowding_distance);
    assert_eq!(INFINITY, cb.crowding_distance);

    // only cd is in the middle. spread is in both dimensions the same
    // (2.0). norm is 1.0 / (spread * #objectives) = 1.0 / 4.0. As we
    // add two times 0.5, the crowding distance should be 1.0.
    assert_eq!(1.0, cd.crowding_distance);

    let f1 = f0.next_front();
    let solutions = f1.iter().collect::<Vec<_>>();
    assert_eq!(1, solutions.len());
    assert_eq!(&c, solutions[0].0);

    assert_eq!(true, f1.next_front().is_empty());
}
