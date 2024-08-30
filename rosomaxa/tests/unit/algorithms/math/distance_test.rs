use super::*;

#[test]
fn can_handle_different_scale() {
    let a = vec![1., 100000.];
    let b = vec![1.01, 101000.];

    let distance = relative_distance(a.into_iter(), b.into_iter());

    let threshold = (0.01 * 2_f64).sqrt() as Float;
    assert!(distance < threshold);
}
