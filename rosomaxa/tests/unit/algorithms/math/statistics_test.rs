use super::*;

#[test]
fn can_get_zero_variance_from_empty_values() {
    assert_eq!(get_variance(&[]), 0.);
    assert_eq!(get_stdev(&[]), 0.);
}
