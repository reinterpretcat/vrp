use super::Remedian;

#[test]
pub fn can_estimate_median() {
    let observations =
        vec![12, 22, 26, 13, 21, 7, 10, 2, 16, 5, 11, 27, 9, 17, 25, 23, 1, 14, 20, 3, 8, 24, 15, 18, 19, 4, 6];
    let mut remedian = Remedian::new(11, |a: &i32, b: &i32| a.cmp(b));
    assert_eq!(remedian.approx_median(), None);

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(17));
}

#[test]
pub fn can_handle_estimate_median_with_low_estimations() {
    let observations = vec![57, 232, 718, 239, 110, 3684, 77, 35, 55, 101];
    let mut remedian = Remedian::new(11, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), None);
}

#[test]
pub fn can_handle_estimate_median_with_base_amount_of_values() {
    let observations = vec![57, 232, 718, 239, 110, 3684, 77, 35, 55, 101];
    let mut remedian = Remedian::new(10, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(110));
}

#[test]
pub fn can_handle_estimate_median_with_base_plus_one_amount_of_values() {
    let observations = vec![57, 232, 718, 239, 110, 3684, 77, 35, 55, 101, 1000];
    let mut remedian = Remedian::new(10, |a: &i32, b: &i32| a.cmp(b));

    observations.iter().cloned().for_each(|o| {
        remedian.add_observation(o);
    });

    assert_eq!(remedian.approx_median(), Some(110));
}

#[test]
pub fn can_estimate_median_with_multiple_buffers() {
    let observations = vec![57, 232, 718, 239, 110];
    let mut remedian = Remedian::new(5, |a: &i32, b: &i32| a.cmp(b));

    (0..100).for_each(|_| {
        observations.iter().cloned().for_each(|o| {
            remedian.add_observation(o);
        });
    });

    assert_eq!(remedian.buffers.len(), 2);
    assert_eq!(remedian.approx_median(), Some(232));
}
