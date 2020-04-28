use super::*;
use crate::helpers::create_empty_problem;

#[test]
fn can_detect_duplicates() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![],
            profiles: vec![
                Profile { name: "my_vehicle".to_string(), profile_type: "car".to_string(), speed: None },
                Profile { name: "my_vehicle".to_string(), profile_type: "truck".to_string(), speed: None },
            ],
        },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1500_duplicated_profiles(&ctx);

    assert_eq!(result.err().map(|err| err.code), Some("E1500".to_string()));
}

#[test]
fn can_detect_empty_profiles() {
    let problem = Problem { fleet: Fleet { vehicles: vec![], profiles: vec![] }, ..create_empty_problem() };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1501_empty_profiles(&ctx);

    assert_eq!(result.err().map(|err| err.code), Some("E1501".to_string()));
}
