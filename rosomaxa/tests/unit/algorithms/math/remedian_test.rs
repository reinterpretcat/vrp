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
