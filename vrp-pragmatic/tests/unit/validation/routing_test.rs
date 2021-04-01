use super::*;
use crate::format::Location;
use crate::helpers::*;

#[test]
fn can_detect_duplicates() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![],
            profiles: vec![
                MatrixProfile { name: "my_vehicle".to_string(), speed: None },
                MatrixProfile { name: "my_vehicle".to_string(), speed: None },
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

#[test]
fn can_detect_mixed_locations() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_index("job1", 0), create_delivery_job("job2", vec![1.0, 0.])],
            relations: None,
        },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1502_no_location_type_mix(&ctx, ctx.coord_index.get_used_types());

    assert_eq!(result.err().map(|err| err.code), Some("E1502".to_string()));
}

#[test]
fn can_detect_missing_matrix_when_indices_used() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_index("job1", 0)], relations: None },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1503_no_matrix_when_indices_used(&ctx, ctx.coord_index.get_used_types());

    assert_eq!(result.err().map(|err| err.code), Some("E1503".to_string()));
}

#[test]
fn can_detect_limit_areas_with_indices() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_index("job1", 0)], relations: None },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    tour_size: None,
                    allowed_areas: Some(vec![AreaLimit {
                        priority: None,
                        outer_shape: vec![
                            Location::new_coordinate(-5., -5.),
                            Location::new_coordinate(5., -5.),
                            Location::new_coordinate(5., 5.),
                            Location::new_coordinate(-5., 5.),
                        ],
                    }]),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1504_limit_areas_cannot_be_used_with_indices(&ctx, ctx.coord_index.get_used_types());

    assert_eq!(result.err().map(|err| err.code), Some("E1504".to_string()));
}

#[test]
fn can_detect_index_mismatch() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_index("job1", 0),
                create_delivery_job_with_index("job2", 1),
                create_delivery_job_with_index("job3", 2),
            ],
            relations: None,
        },
        ..create_empty_problem()
    };
    let matrices = vec![Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1; 4],
        distances: vec![1; 4],
        error_codes: None,
    }];
    let ctx = ValidationContext::new(&problem, Some(&matrices));

    let result = check_e1505_index_size_mismatch(&ctx);

    assert_eq!(result.err().map(|err| err.code), Some("E1505".to_string()));
}

#[test]
fn can_detect_missing_profile() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![
                VehicleType { profile: create_vehicle_profile_with_name("car"), ..create_default_vehicle_type() },
                VehicleType { profile: create_vehicle_profile_with_name("truck"), ..create_default_vehicle_type() },
            ],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = check_e1506_profiles_exist(&ctx);

    assert_eq!(result.err().map(|err| err.code), Some("E1506".to_string()));
}
