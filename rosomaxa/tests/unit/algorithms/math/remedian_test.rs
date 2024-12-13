use super::Remedian;

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
