use super::*;
use crate::helpers::*;

fn validate_result(ctx: &ValidationContext) -> Option<FormatError> {
    let result = validate_relations(ctx);

    result.err().map(|result| {
        assert_eq!(result.errors.len(), 1);
        result.errors.first().cloned().unwrap()
    })
}

parameterized_test! {can_detect_relation_errors, (job_ids, vehicle_id, shift_index, expected), {
    can_detect_relation_errors_impl(
        job_ids.iter().map(|id| id.to_string()).collect(),
        vehicle_id.to_string(),
        shift_index,
        expected,
    );
}}

can_detect_relation_errors! {
    case01: (vec!["job2"], "my_vehicle_1", None, None),
    case02: (vec!["job1", "job2", "job3"], "my_vehicle_1", None, Some(("E1200", "job1, job3"))),
    case03: (vec!["job2"], "my_vehicle_2", None, Some(("E1201", "my_vehicle_2"))),

    case04: (Vec::<&str>::default(), "my_vehicle_1", None, Some(("E1202", "jobs list"))),
    case05: (vec!["departure", "arrival"], "my_vehicle_1", None, Some(("E1202", "jobs list"))),

    case06: (vec!["job2"], "my_vehicle_1", Some(0), None),
    case07: (vec!["job2"], "my_vehicle_1", Some(1), Some(("E1205", "my_vehicle_1"))),

    case08: (vec!["departure", "job2", "break"], "my_vehicle_1", None, Some(("E1206", "break"))),
    case09: (vec!["departure", "job2", "reload"], "my_vehicle_1", None, Some(("E1206", "reload"))),
}

fn can_detect_relation_errors_impl(
    job_ids: Vec<String>,
    vehicle_id: String,
    shift_index: Option<usize>,
    expected: Option<(&str, &str)>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job2", (1., 0.))],
            relations: Some(vec![Relation {
                type_field: RelationType::Strict,
                jobs: job_ids,
                vehicle_id,
                shift_index,
            }]),
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    if let Some((code, action)) = expected {
        assert_eq!(result.clone().map(|err| err.code), Some(code.to_string()));
        assert!(result.map_or("".to_string(), |err| err.action).contains(action));
    } else {
        assert!(result.is_none());
    }
}

parameterized_test! {can_detect_multi_place_time_window_jobs, (relation_type, expected), {
    can_detect_multi_place_time_window_jobs_impl(relation_type, expected);
}}

can_detect_multi_place_time_window_jobs! {
    case01: (RelationType::Strict, Some(())),
    case02: (RelationType::Sequence, Some(())),
    case03: (RelationType::Any, Some(())),
}

fn can_detect_multi_place_time_window_jobs_impl(relation_type: RelationType, expected: Option<()>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (1., 0.), vec![(10, 20), (30, 40)], 10.),
                create_delivery_job("job2", (1., 0.)),
                Job {
                    deliveries: Some(vec![JobTask {
                        places: vec![create_job_place((1., 0.), None), create_job_place((2., 0.), None)],
                        ..create_task((1., 0.), None)
                    }]),
                    ..create_job("job3")
                },
            ],
            relations: Some(vec![Relation {
                type_field: relation_type,
                jobs: vec!["job1".to_string(), "job2".to_string(), "job3".to_string()],
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    match (&result, &expected) {
        (Some(error), Some(_)) => {
            assert_eq!(error.code, "E1203");
            assert!(error.action.contains("job1, job3"))
        }
        (None, None) => {}
        _ => panic!("{:?} vs {}", result, expected.is_some()),
    }
}

parameterized_test! {can_detect_multi_vehicle_assignment, (relations, expected), {
    can_detect_multi_vehicle_assignment_impl(relations, expected);
}}

can_detect_multi_vehicle_assignment! {
    case01: (vec![("job1", "car_1")], None),
    case02: (vec![("job1", "car_1"), ("job1", "car_1")], None),
    case03: (vec![("job1", "car_1"), ("job2", "car_1")], None),
    case04: (vec![("job1", "car_1"), ("job1", "truck_1")], Some("job1")),
}

fn can_detect_multi_vehicle_assignment_impl(relations: Vec<(&str, &str)>, expected: Option<&str>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (2., 0.))],
            relations: Some(
                relations
                    .iter()
                    .map(|(job_id, vehicle_id)| Relation {
                        type_field: RelationType::Any,
                        jobs: vec![job_id.to_string()],
                        vehicle_id: vehicle_id.to_string(),
                        shift_index: None,
                    })
                    .collect(),
            ),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("car"), create_default_vehicle("truck")],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    match (&result, &expected) {
        (Some(error), Some(jobs)) => {
            assert_eq!(error.code, "E1204");
            assert!(error.action.contains(jobs))
        }
        (None, None) => {}
        _ => panic!("{:?} vs {}", result, expected.is_some()),
    }
}

parameterized_test! {can_detect_incomplete_multi_job_in_relation, (relation_type, jobs, expected), {
    can_detect_incomplete_multi_job_in_relation_impl(relation_type,
        jobs.iter().map(|job| job.to_string()).collect(),
        expected.map(|result| result.to_string()));
}}

can_detect_incomplete_multi_job_in_relation! {
    case01: (RelationType::Any, &["job1"], Some("E1207")),
    case02: (RelationType::Sequence, &["job1"], Some("E1207")),
    case03: (RelationType::Strict, &["job1"], Some("E1207")),
    case04: (RelationType::Any, &["job1", "job1"], Option::<String>::None),
}

fn can_detect_incomplete_multi_job_in_relation_impl(
    relation_type: RelationType,
    jobs: Vec<String>,
    expected: Option<String>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_pickup_delivery_job("job1", (1., 0.), (2., 0.))],
            relations: Some(vec![Relation {
                type_field: relation_type,
                jobs,
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.map(|err| err.code), expected);
}
