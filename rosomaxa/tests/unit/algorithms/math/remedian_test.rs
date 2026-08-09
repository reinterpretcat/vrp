use super::Remedian;
use std::cmp::Ordering;
use std::ops::ControlFlow;

#[test]
pub fn can_estimate_median() {
    let observations =
        [12, 22, 26, 13, 21, 7, 10, 2, 16, 5, 11, 27, 9, 17, 25, 23, 1, 14, 20, 3, 8, 24, 15, 18, 19, 4, 6];
    let mut remedian = Remedian::new(11, 2, |a: &i32, b: &i32| a.cmp(b));
    assert_eq!(remedian.approx_median(), None);

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(12));
}

#[test]
pub fn can_handle_estimate_median_with_low_estimations() {
    let observations = [57, 232, 718, 239, 110, 3684, 77, 35, 55, 300];
    let mut remedian = Remedian::new(11, 2, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(110));
}

#[test]
pub fn can_handle_estimate_median_with_base_amount_of_values() {
    let observations = [57, 232, 718, 239, 110, 3684, 77, 35, 55, 101];
    let mut remedian = Remedian::new(11, 2, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(101));
}

#[test]
pub fn can_handle_estimate_median_with_base_plus_one_amount_of_values() {
    let observations = [57, 232, 718, 239, 110, 3684, 77, 35, 55, 101, 1000];
    let mut remedian = Remedian::new(11, 2, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(110));
}

#[test]
pub fn can_estimate_median_with_multiple_buffers() {
    let observations = [57, 232, 718, 239, 110];
    let mut remedian = Remedian::new(5, 2, |a: &i32, b: &i32| a.cmp(b));

    (0..100).for_each(|_| {
        observations.iter().cloned().for_each(|o| {
            remedian.add_observation(o);
        });
    });

    assert_eq!(remedian.buffers.len(), 2);
    assert_eq!(remedian.approx_median(), Some(232));
}

#[test]
pub fn can_handle_estimate_median_with_more_data() {
    let observations = [
        17, 72, 97, 8, 32, 15, 63, 97, 57, 60, 83, 48, 100, 26, 12, 62, 3, 49, 55, 77, 97, 98, 0, 89, 57, 34, 92, 29,
        75, 13,
    ];
    let expected = vec![
        17, 17, 17, 17, 17, 17, 17, 32, 32, 57, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 55, 55, 55, 55, 55, 55, 55,
        55, 55,
    ];

    let mut medians = Vec::new();
    let mut remedian = Remedian::new(11, 2, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
        medians.push(remedian.approx_median().unwrap_or_default());
    });

    assert_eq!(medians, expected);
}

#[test]
pub fn can_keep_estimate_equivalent_to_weighted_buffer_median() {
    let mut seed = 42_u64;

    for (base, exponent) in [(1_usize, 3_usize), (3, 3), (5, 2), (11, 2)] {
        let mut remedian = Remedian::new(base, exponent, |a: &u64, b: &u64| a.cmp(b));
        let capacity = base.pow(exponent as u32);

        for _ in 0..capacity {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            assert!(remedian.add_observation(seed % 17));
            assert_eq!(remedian.approx_median(), get_weighted_buffer_median(&remedian));
        }

        assert!(!remedian.add_observation(100));
        assert_eq!(remedian.approx_median(), get_weighted_buffer_median(&remedian));
    }

    let mut remedian = Remedian::new(11, 7, |a: &u64, b: &u64| a.cmp(b));
    for _ in 0..10_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        assert!(remedian.add_observation(seed % 17));
        assert_eq!(remedian.approx_median(), get_weighted_buffer_median(&remedian));
    }
}

fn get_weighted_buffer_median<F>(remedian: &Remedian<u64, F>) -> Option<u64>
where
    F: Fn(&u64, &u64) -> Ordering,
{
    let mut values = remedian
        .buffers
        .iter()
        .enumerate()
        .flat_map(|(idx, buffer)| buffer.iter().map(move |value| (*value, (remedian.base as u64).pow(idx as u32))))
        .collect::<Vec<_>>();
    values.sort_by_key(|(value, _)| *value);

    values
        .iter()
        .try_fold(0, |running_weight, (value, weight)| {
            let running_weight = running_weight + weight;
            if running_weight >= remedian.count as u64 / 2 {
                ControlFlow::Break(*value)
            } else {
                ControlFlow::Continue(running_weight)
            }
        })
        .break_value()
}
