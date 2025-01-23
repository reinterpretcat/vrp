use super::*;

#[test]
fn can_handle_relative_distance_empty() {
    let a: Vec<Float> = vec![];
    let b: Vec<Float> = vec![];
    let result = relative_distance(a.into_iter(), b.into_iter());
    assert_eq!(result, 0.0);
}

#[test]
fn can_handle_relative_distance_single_element_equal() {
    let a = vec![1.0];
    let b = vec![1.0];

    assert_eq!(relative_distance(a.into_iter(), b.into_iter()), 0.0);
}

#[test]
fn can_handle_relative_distance_single_element_different() {
    let a = vec![1.0];
    let b = vec![2.0];
    let result = relative_distance(a.into_iter(), b.into_iter());
    assert!((result - 0.5).abs() < f64::EPSILON);
}

#[test]
fn can_handle_relative_distance_different_scale() {
    let a = vec![1., 100000.];
    let b = vec![1.01, 101000.];

    assert!((relative_distance(a.into_iter(), b.into_iter()) - 0.014).abs() < 0.0001);
}

#[test]
fn can_handle_relative_distance_identical_vectors() {
    let a = vec![1.0, 1.0, 1.0];
    let b = vec![1.0, 1.0, 1.0];

    assert_eq!(relative_distance(a.into_iter(), b.into_iter()), 0.);
}

#[test]
fn can_handle_relative_distance_opposite_vectors() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![-1.0, -2.0, -3.0];

    assert!((relative_distance(a.into_iter(), b.into_iter()) - 12_f64.sqrt()).abs() < 0.0001);
}

#[test]
fn can_handle_relative_distance_zero_vector() {
    let a = vec![0.0, 0.0, 0.0];
    let b = vec![1.0, 1.0, 1.0];

    assert!((relative_distance(a.into_iter(), b.into_iter()) - 1.7320).abs() < 0.0001);
}

#[test]
fn can_handle_relative_distance_large_vectors() {
    let a: Vec<f64> = (0..100).map(|x| x as f64).collect();
    let b: Vec<f64> = (0..100).map(|x| (x + 1) as f64).collect();

    assert!((relative_distance(a.into_iter(), b.into_iter()) - 1.278).abs() < 0.001);
}
