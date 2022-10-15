use super::*;

fn make(data: &[Cost]) -> InsertionCost {
    InsertionCost::new(data)
}

#[test]
fn can_use_big_sizes_for_insertion_costs() {
    let cost = make(&[0., 0., 1., 0., 0., 2., 0., 0., 3., 0., 0., 4.]);

    assert_eq!(cost.data[0], 0.);
    assert_eq!(cost.data[1], 0.);
    assert_eq!(cost.data[2], 1.);
    assert_eq!(cost.data[3], 0.);
    assert_eq!(cost.data[5], 2.);
    assert_eq!(cost.data[8], 3.);
    assert_eq!(cost.data[11], 4.);
}

#[test]
fn can_compare_insertion_costs() {
    assert_eq!(make(&[1., 0., 0.]), make(&[1., 0., 0.]));
    assert_eq!(make(&[0., 1., 0.]), make(&[0., 1., 0.]));
    assert_eq!(make(&[0., 1.]), make(&[0., 1., 0.]));
    assert_eq!(make(&[0., 1., 0.]), make(&[0., 1.]));
    assert_eq!(make(&[0., 0., 1.]), make(&[0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 0.]));

    assert!(make(&[0., 1., 0.]) > make(&[0., 0., 0.]));
    assert!(make(&[0., 0., 0.]) < make(&[0., 1., 0.]));

    assert!(make(&[0., 1.]) < make(&[0., 1., 1.]));
    assert!(make(&[0., 0., 1.]) < make(&[0., 0., 1., 0., 0., 0., 0., 0., 0., 0., 0., 1.]));
}
