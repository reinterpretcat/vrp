use super::*;
use crate::utils::DefaultRandom;

#[test]
fn can_sample_from_range() {
    let random = Arc::new(DefaultRandom::default());
    let amount = 5;
    let numbers = SelectionSamplingIterator::new((0..100).into_iter(), amount, random);

    let result = numbers.collect::<Vec<_>>();

    assert_eq!(result.len(), amount);
    result.windows(2).for_each(|item| match item {
        &[prev, next] => assert!(prev < next),
        _ => unreachable!(),
    });
    result.windows(2).any(|item| match item {
        &[prev, next] => prev + 1 < next,
        _ => false,
    });
}
