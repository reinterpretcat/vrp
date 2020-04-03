use super::*;
use crate::helpers::*;

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
}

fn can_detect_relation_errors_impl(job_ids: Vec<String>, vehicle_id: String, expected: Option<(&str, &str)>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job2", vec![1., 0.])],
            relations: Some(vec![Relation {
                type_field: RelationType::Any,
                jobs: job_ids,
                vehicle_id,
                shift_index: None,
            }]),
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("vehicle")], profiles: vec![] },
        ..create_empty_problem()
    };
    let ctx = ValidationContext::new(&problem, None);

    let result = validate_relations(&ctx);
    let result = result.err().map(|errors| {
        assert_eq!(errors.len(), 1);
        errors.first().cloned().unwrap()
    });

    if let Some((code, action)) = expected {
        assert_eq!(result.clone().map(|err| err.code), Some(code.to_string()));
        assert!(result.map_or("".to_string(), |err| err.action).contains(action));
    } else {
        assert!(result.is_none());
    }
}
