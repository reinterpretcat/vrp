use super::*;
use crate::helpers::generate::{create_test_job, create_test_vehicle_type};
use vrp_pragmatic::format::problem::{Fleet, MatrixProfile, Plan};

#[test]
fn can_get_locations_serialized() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_test_job(1., 1.), create_test_job(1., 0.)], relations: None },
        fleet: Fleet { vehicles: vec![create_test_vehicle_type()], profiles: vec![] },
        objectives: None,
    };

    let locations = get_locations_serialized(&problem).unwrap().replace(" ", "").replace("\n", "");

    assert_eq!(locations, r#"[{"lat":1.0,"lng":1.0},{"lat":1.0,"lng":0.0},{"lat":0.0,"lng":0.0}]"#);
}

#[test]
fn can_get_solution_serialized() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_test_job(1., 0.)], relations: None },
        fleet: Fleet {
            vehicles: vec![create_test_vehicle_type()],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        objectives: None,
    };
    let problem = Arc::new(problem.read_pragmatic().unwrap());

    let solution = get_solution_serialized(problem, Config::default()).unwrap().replace(" ", "").replace("\n", "");

    assert!(solution.starts_with("{"));
    assert!(solution.ends_with("}"));
    assert!(solution.contains("statistic"));
    assert!(solution.contains("tours"));
    assert!(solution.contains("stops"));
}
