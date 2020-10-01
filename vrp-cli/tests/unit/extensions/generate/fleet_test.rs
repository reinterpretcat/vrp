use super::*;
use crate::helpers::generate::create_test_vehicle_type;
use vrp_pragmatic::format::problem::{Plan, Profile};

#[test]
fn can_generate_fleet_of_specific_size() {
    let prototype = Problem {
        plan: Plan { jobs: vec![], relations: None },
        fleet: Fleet {
            vehicles: vec![create_test_vehicle_type()],
            profiles: vec![Profile {
                name: "normal_car".to_string(),
                profile_type: "car_type".to_string(),
                speed: None,
            }],
        },
        objectives: None,
        config: None,
    };

    let generated = generate_fleet(&prototype, 2).unwrap();

    assert_eq!(generated.vehicles.len(), 2);
    assert_eq!(generated.profiles.len(), 1);
}
