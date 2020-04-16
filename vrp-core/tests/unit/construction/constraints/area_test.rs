use super::*;

#[test]
fn can_check_single_job() {
    // TODO
}

#[test]
fn can_check_single_job_with_multiple_locations() {
    // TODO
}

#[test]
fn can_check_multi_job() {
    // TODO
}

#[test]
fn can_check_location_in_area() {
    let polygon = vec![(-1., -1.), (-1., 1.), (1., 1.), (1., -1.)];
    assert_eq!(is_location_in_area(&(0., 0.), &polygon), true);
    assert_eq!(is_location_in_area(&(2., 0.), &polygon), false);

    let polygon = vec![(1., 3.), (2., 8.), (5., 4.), (5., 9.), (7., 5.), (13., 1.), (3., 1.)];
    assert_eq!(is_location_in_area(&(5.5, 7.), &polygon), true);
    assert_eq!(is_location_in_area(&(4.5, 7.), &polygon), false);

    let polygon = vec![
        (52.499148, 13.485196),
        (52.498600, 13.480000),
        (52.503800, 13.474680),
        (52.510000, 13.468270),
        (52.510788, 13.466904),
        (52.512116, 13.465350),
        (52.512000, 13.467000),
        (52.513579, 13.471027),
        (52.512938, 13.472668),
        (52.511829, 13.474922),
        (52.507945, 13.480124),
        (52.509082, 13.482892),
        (52.536026, 13.490519),
        (52.534470, 13.499703),
        (52.499148, 13.485196),
    ];
    assert_eq!(is_location_in_area(&(52.508956, 13.483328), &polygon), true);
    assert_eq!(is_location_in_area(&(52.505, 13.48), &polygon), true);

    let polygon =
        vec![(52.481171, 13.4107070), (52.480248, 13.4101200), (52.480237, 13.4062790), (52.481161, 13.4062610)];
    assert_eq!(is_location_in_area(&(52.480890, 13.4081030), &polygon), true);
}
