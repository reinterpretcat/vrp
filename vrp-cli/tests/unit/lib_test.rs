use super::*;
use crate::helpers::generate::{create_empty_plan, create_test_job, create_test_vehicle_type};
use vrp_pragmatic::format::problem::{Fleet, MatrixProfile, Plan};
use vrp_pragmatic::format::MultiFormatError;

#[test]
fn can_get_locations_serialized() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_test_job(1., 1.), create_test_job(1., 0.)], ..create_empty_plan() },
        fleet: Fleet { vehicles: vec![create_test_vehicle_type()], profiles: vec![], resources: None },
        objectives: None,
    };

    let locations = get_locations_serialized(&problem).unwrap().replace([' ', '\n'], "");

    assert_eq!(locations, r#"[{"lat":1.0,"lng":1.0},{"lat":1.0,"lng":0.0},{"lat":0.0,"lng":0.0}]"#);
}

#[test]
fn can_get_solution_serialized() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_test_job(1., 0.)], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![create_test_vehicle_type()],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
            resources: None,
        },
        objectives: None,
    };
    let problem = Arc::new(problem.read_pragmatic().unwrap());

    let solution = get_solution_serialized(problem, Config::default()).unwrap().replace([' ', '\n'], "");

    assert!(solution.starts_with('{'));
    assert!(solution.ends_with('}'));
    assert!(solution.contains("statistic"));
    assert!(solution.contains("tours"));
    assert!(solution.contains("stops"));
}

#[test]
fn can_get_errors_serialized() {
    let errors = vec![
        FormatError::new("code0".to_string(), "cause0".to_string(), "action0".to_string()),
        FormatError::new("code1".to_string(), "cause1".to_string(), "action1".to_string()),
    ];

    let result = MultiFormatError::from(errors).to_string();

    assert_eq!(
        "code0, cause: \'cause0\', action: \'action0\'.\ncode1, cause: \'cause1\', action: \'action1\'.",
        result
    );
}

#[test]
fn can_get_config_error() {
    let result = serialize_as_config_error("some error");

    assert!(result.starts_with('{'));
    assert!(result.ends_with('}'));
    assert!(result.contains("some error"));
    assert!(result.contains("E0004"));
    assert!(result.contains("cannot read config"));
}
