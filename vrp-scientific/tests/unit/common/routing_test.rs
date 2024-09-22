use super::*;
use vrp_core::models::common::Profile;

fn get_index() -> CoordIndex {
    let mut index = CoordIndex::default();
    index.collect((0, 0));
    index.collect((2, 1));

    index
}

#[test]
fn can_create_transport_without_rounding() {
    let index = get_index();
    let logger: InfoLogger = Arc::new(|_| ());

    let transport = index.create_transport(false, &logger).unwrap();

    assert!((transport.distance_approx(&Profile::new(0, None), 0, 1) - 2.23606).abs() < 1E-5);
}

#[test]
fn can_create_transport_with_rounding() {
    let index = get_index();
    let logger: InfoLogger = Arc::new(|_| ());

    let transport = index.create_transport(true, &logger).unwrap();

    assert_eq!(transport.distance_approx(&Profile::new(0, None), 0, 1), 2.);
}
