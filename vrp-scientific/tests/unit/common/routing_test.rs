use super::*;
use vrp_core::models::common::Profile;

fn get_index() -> CoordIndex {
    let mut index = CoordIndex::default();
    index.collect((0, 0));
    index.collect((2, 1));

    index
}

#[test]
fn can_create_transport_with_routing_mode_scale() {
    let index = get_index();

    let transport = index.create_transport(RoutingMode::ScaleNoRound(1000.)).unwrap();

    assert_eq!(transport.distance_approx(&Profile::new(0, None), 0, 1), 2236);
}

#[test]
fn can_create_transport_with_routing_mode_simple() {
    let index = get_index();

    let transport = index.create_transport(RoutingMode::Simple).unwrap();

    assert_eq!(transport.distance_approx(&Profile::new(0, None), 0, 1), 2);
}
