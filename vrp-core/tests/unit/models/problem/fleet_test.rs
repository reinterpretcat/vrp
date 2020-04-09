use crate::helpers::models::problem::{test_driver, test_vehicle, FleetBuilder};

#[test]
fn fleet_creates_unique_profiles_from_vehicles() {
    let profile1 = 0;
    let profile2 = 1;

    assert_eq!(
        FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicle(test_vehicle(profile1))
            .add_vehicle(test_vehicle(profile2))
            .add_vehicle(test_vehicle(profile1))
            .build()
            .profiles,
        vec![profile1.to_owned(), profile2.to_owned()]
    )
}
