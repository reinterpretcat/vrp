use super::*;
use crate::helpers::*;

parameterized_test! {can_detect_reserved_ids, (job_id, expected), {
    can_detect_reserved_ids_impl(job_id.to_string(), expected);
}}

can_detect_reserved_ids! {
    case01: ("job1", None),
    case02: ("departure", Some("departure")),
    case03: ("arrival", Some("arrival")),
    case04: ("break", Some("break")),
    case05: ("reload", Some("reload")),
}

fn can_detect_reserved_ids_impl(job_id: String, expected: Option<&str>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job(job_id.as_str(), vec![1., 0.])],
            relations: Some(vec![Relation {
                type_field: RelationType::Strict,
                jobs: vec![job_id],
                vehicle_id: "vehicle_1".to_string(),
                shift_index: None,
            }]),
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("vehicle")], profiles: vec![] },
        ..create_empty_problem()
    };

    let result = check_e1104_no_reserved_ids(&ValidationContext::new(&problem, None)).err();

    if let Some(action) = expected {
        assert_eq!(result.clone().map(|err| err.code), Some("E1104".to_string()));
        assert!(result.map_or("".to_string(), |err| err.action).contains(action));
    } else {
        assert!(result.is_none());
    }
}
