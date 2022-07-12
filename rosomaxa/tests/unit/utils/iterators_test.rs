use super::*;
use crate::utils::DefaultRandom;

#[test]
fn can_sample_from_large_range() {
    let random = Arc::new(DefaultRandom::default());
    let amount = 5;

    let numbers = SelectionSamplingIterator::new(0..100, amount, random).collect::<Vec<_>>();

    assert_eq!(numbers.len(), amount);
    numbers.windows(2).for_each(|item| match item {
        &[prev, next] => assert!(prev < next),
        _ => unreachable!(),
    });
    numbers.windows(2).any(|item| match item {
        &[prev, next] => prev + 1 < next,
        _ => false,
    });
}

#[test]
fn can_sample_from_same_range() {
    let amount = 5;
    let random = Arc::new(DefaultRandom::default());

    let numbers = SelectionSamplingIterator::new(0..amount, amount, random).collect::<Vec<_>>();

    assert_eq!(numbers, vec![0, 1, 2, 3, 4])
}

#[test]
fn can_sample_from_smaller_range() {
    let amount = 5;
    let random = Arc::new(DefaultRandom::default());

    let numbers = SelectionSamplingIterator::new(0..3, amount, random).collect::<Vec<_>>();

    assert_eq!(numbers, vec![0, 1, 2])
}
