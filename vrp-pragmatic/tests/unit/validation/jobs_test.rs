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
        plan: Plan { jobs: vec![create_delivery_job(job_id.as_str(), vec![1., 0.])], relations: None },
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

#[test]
fn can_detect_empty_job() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job {
                id: "job1".to_string(),
                pickups: None,
                deliveries: Some(vec![]),
                replacements: None,
                services: None,
                priority: None,
                skills: None,
            }],
            relations: None,
        },
        ..create_empty_problem()
    };

    let result = check_e1105_empty_jobs(&ValidationContext::new(&problem, None)).err();

    assert_eq!(result.clone().map(|err| err.code), Some("E1105".to_string()));
    assert!(result.map_or("".to_string(), |err| err.action).contains("job1"));
}
