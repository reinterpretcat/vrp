use super::*;
use crate::helpers::generate::create_test_vehicle_type;
use vrp_pragmatic::format::problem::{MatrixProfile, Plan};

#[test]
fn can_generate_fleet_of_specific_size() {
    let prototype = Problem {
        plan: Plan { jobs: vec![], relations: None },
        fleet: Fleet {
            vehicles: vec![create_test_vehicle_type()],
            profiles: vec![MatrixProfile { name: "normal_car".to_string(), speed: None }],
        },
        objectives: None,
    };

    let generated = generate_fleet(&prototype, 2);

    assert_eq!(generated.vehicles.len(), 2);
    assert_eq!(generated.profiles.len(), 1);
}
