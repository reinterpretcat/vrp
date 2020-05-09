use super::*;

fn round(value: f64) -> f64 {
    (value * 1000.).round() / 1000.
}

#[test]
pub fn can_calculate_distance_between_points() {
    let a = Point::new(3., 2.);
    let b = Point::new(9., 7.);

    assert_eq!(round(a.distance_to_point(&b)), 7.81);
}

#[test]
pub fn can_calculate_distance_to_line() {
    let a = Point::new(0., 2.);
    let b = Point::new(5., 8.);
    let c = Point::new(-3., 7.);

    assert_eq!(round(c.distance_to_line(&a, &b)), 5.506);
}
