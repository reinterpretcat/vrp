use super::*;

use crate::helpers::models::problem::{test_driver, test_vehicle};
use std::iter::FromIterator;

#[test]
fn fleet_creates_unique_profiles_from_vehicles() {
    let profile1 = "car";
    let profile2 = "truck";
    let drivers = vec![test_driver()];
    let vehicles = vec![
        test_vehicle(profile1),
        test_vehicle(profile2),
        test_vehicle(profile1),
    ];

    assert_eq!(
        Fleet::new(drivers, vehicles).profiles,
        vec![profile1.to_owned(), profile2.to_owned()]
    )
}
