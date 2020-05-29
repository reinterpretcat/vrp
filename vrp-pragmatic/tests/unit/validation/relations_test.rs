use super::*;
use crate::helpers::*;

fn validate_result(ctx: &ValidationContext) -> Option<FormatError> {
    let result = validate_relations(&ctx);

    result.err().map(|errors| {
        assert_eq!(errors.len(), 1);
        errors.first().cloned().unwrap()
    })
}

parameterized_test! {can_detect_relation_errors, (job_ids, vehicle_id, expected), {
    can_detect_relation_errors_impl(
        job_ids.iter().map(|id| id.to_string()).collect(),
        vehicle_id.to_string(),
        expected,
    );
}}

can_detect_relation_errors! {
    case01: (vec!["job2"], "vehicle_1", None),
    case02: (vec!["job1", "job2", "job3"], "vehicle_1", Some(("E1200", "job1, job3"))),
    case03: (vec!["job2"], "vehicle_2", Some(("E1201", "vehicle_2"))),
    case04: (Vec::<&str>::default(), "vehicle_1", Some(("E1202", "jobs list"))),
    case05: (vec!["departure", "arrival"], "vehicle_1", Some(("E1202", "jobs list"))),
}

fn can_detect_relation_errors_impl(job_ids: Vec<String>, vehicle_id: String, expected: Option<(&str, &str)>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job2", vec![1., 0.])],
            relations: Some(vec![Relation {
                type_field: RelationType::Strict,
                jobs: job_ids,
                vehicle_id,
                shift_index: None,
            }]),
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("vehicle")], profiles: vec![] },
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None));

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
    case03: (RelationType::Any, None),
}

fn can_detect_multi_place_time_window_jobs_impl(relation_type: RelationType, expected: Option<()>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![1., 0.], vec![(10, 20), (30, 40)], 10.),
                create_delivery_job("job2", vec![1., 0.]),
                Job {
                    deliveries: Some(vec![JobTask {
                        places: vec![create_job_place(vec![1., 0.]), create_job_place(vec![2., 0.])],
                        ..create_task(vec![1., 0.])
                    }]),
                    ..create_job("job3")
                },
            ],
            relations: Some(vec![Relation {
                type_field: relation_type,
                jobs: vec!["job1".to_string(), "job2".to_string(), "job3".to_string()],
                vehicle_id: "vehicle_1".to_string(),
                shift_index: None,
            }]),
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("vehicle")], profiles: vec![] },
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None));

    match (&result, &expected) {
        (Some(error), Some(_)) => {
            assert_eq!(error.code, "E1203");
            assert!(error.action.contains("job1, job3"))
        }
        (None, None) => {}
        _ => panic!(format!("{:?} vs {}", result, expected.is_some())),
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
            jobs: vec![create_delivery_job("job1", vec![1.0, 0.]), create_delivery_job("job2", vec![2.0, 0.])],
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
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("car"), create_default_vehicle("truck")],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result = validate_result(&ValidationContext::new(&problem, None));

    match (&result, &expected) {
        (Some(error), Some(jobs)) => {
            assert_eq!(error.code, "E1204");
            assert!(error.action.contains(jobs))
        }
        (None, None) => {}
        _ => panic!(format!("{:?} vs {}", result, expected.is_some())),
    }
}
