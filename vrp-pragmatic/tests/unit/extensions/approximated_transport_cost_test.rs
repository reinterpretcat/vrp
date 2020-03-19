use crate::extensions::approximated_transport_cost::get_distance;
use crate::extensions::ApproximatedTransportCost;
use crate::json::Location;
use vrp_core::models::problem::TransportCost;

#[test]
fn can_calculate_distance_between_two_locations() {
    let l1 = Location { lat: 52.52599, lng: 13.45413 };
    let l2 = Location { lat: 52.5165, lng: 13.3808 };

    let distance = get_distance(&l1, &l2);

    assert_eq!(distance.round(), 5078.);
}

#[test]
fn can_create_transport_costs() {
    let locations = vec![
        Location { lat: 52.52599, lng: 13.45413 },
        Location { lat: 52.5225, lng: 13.4095 },
        Location { lat: 52.5165, lng: 13.3808 },
    ];
    let speed = 15.;
    let costs = ApproximatedTransportCost::new(&locations, speed);

    vec![(0, 1, 3048.), (1, 2, 2056.), (2, 0, 5078.)].into_iter().for_each(|(from, to, expected)| {
        let distance = costs.distance(0, from, to, 0.);
        let duration = costs.duration(0, from, to, 0.);

        assert_eq!(distance.round(), expected);
        assert_eq!(duration.round(), (distance / speed).round());
    });
}
