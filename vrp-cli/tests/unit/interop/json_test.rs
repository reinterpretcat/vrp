use super::*;
use crate::helpers::generate::{SIMPLE_PROBLEM, create_empty_plan, create_test_job, create_test_vehicle_type};
use vrp_pragmatic::format::problem::{Fleet, MatrixProfile, Plan};

const ONE_GENERATION_CONFIG: &str = r#"{"termination": {"maxGenerations": 1}}"#;

fn codes_of(err: &MultiFormatError) -> Vec<String> {
    err.errors.iter().map(|error| error.code.clone()).collect()
}

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
fn can_get_routing_locations() {
    let locations = routing_locations(SIMPLE_PROBLEM).unwrap();

    assert!(locations.starts_with('['));
    assert!(locations.ends_with(']'));
    assert!(locations.len() > 2);
}

#[test]
fn can_validate_simple_problem() {
    assert!(validate(SIMPLE_PROBLEM, &[]).is_ok());
}

#[test]
fn can_detect_malformed_problem() {
    let err = validate("", &[]).unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0000"]);
}

#[test]
fn can_detect_malformed_matrix() {
    let err = validate(SIMPLE_PROBLEM, &["".to_string()]).unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0001"]);
}

#[test]
fn can_report_problem_and_matrix_errors_together() {
    let err = validate("", &["".to_string()]).unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0000", "E0001"]);
}

#[test]
fn can_solve_simple_problem() {
    let solution = solve(SIMPLE_PROBLEM, &[], ONE_GENERATION_CONFIG).unwrap();

    assert!(solution.starts_with('{'));
    assert!(solution.ends_with('}'));
    assert!(solution.contains("statistic"));
}

#[test]
fn can_detect_malformed_config_on_solve() {
    let err = solve(SIMPLE_PROBLEM, &[], "not a json").unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0004"]);
}

#[test]
fn can_validate_before_solving() {
    // NOTE: an empty problem is rejected while reading, so solving must not be attempted
    let err = solve("", &[], ONE_GENERATION_CONFIG).unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0000"]);
}

#[test]
fn can_detect_unknown_convert_format() {
    let err = convert("unknown", &[]).unwrap_err();

    assert_eq!(codes_of(&err), vec!["E0000"]);
}

#[test]
fn can_serialize_errors_as_json() {
    let err = validate("", &[]).unwrap_err();

    let json = err.to_json();

    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));
    assert!(json.contains("E0000"));
    assert!(json.contains("cause"));
    assert!(json.contains("action"));
}
