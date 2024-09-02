use super::*;
use crate::format::Location;
use vrp_core::models::common::{Duration, Profile};
use vrp_core::models::problem::{create_matrix_transport_cost, MatrixData};

fn get_test_locations() -> Vec<Location> {
    vec![
        Location::Coordinate { lat: 52.52599, lng: 13.45413 },
        Location::Coordinate { lat: 52.5225, lng: 13.4095 },
        Location::Coordinate { lat: 52.5165, lng: 13.3808 },
    ]
}

#[test]
fn can_calculate_distance_between_two_locations() {
    let l1 = Location::Coordinate { lat: 52.52599, lng: 13.45413 };
    let l2 = Location::Coordinate { lat: 52.5165, lng: 13.3808 };

    let distance = get_haversine_distance(&l1, &l2);

    assert_eq!(distance.round(), 5078.);
}

#[test]
fn can_use_approximated_with_matrix_costs() {
    let profile = Profile::default();
    let locations = get_test_locations();
    let speed = 10.;
    let approx_data = get_approx_transportation(&locations, &[speed]);
    assert_eq!(approx_data.len(), 1);

    let (durations, distances) = approx_data.first().unwrap().clone();
    let costs = create_matrix_transport_cost(vec![MatrixData::new(profile.index, None, durations, distances)])
        .expect("Cannot create matrix transport costs");

    vec![(0, 1, 3048), (1, 2, 2056), (2, 0, 5078)].into_iter().for_each(|(from, to, expected)| {
        let distance = costs.distance_approx(&profile, from, to);
        let duration = costs.duration_approx(&profile, from, to);

        assert_eq!(distance, expected);
        assert_eq!(duration, (distance as Float / speed).round() as Duration);
    });
}
