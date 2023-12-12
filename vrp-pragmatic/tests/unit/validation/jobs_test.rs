use super::*;
use crate::helpers::*;

fn assert_result(code: &str, action: &str, result: Option<FormatError>) {
    assert_eq!(result.clone().map(|err| err.code), Some(code.to_string()));
    assert!(result.map_or("".to_string(), |err| err.action).contains(action));
}

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
        plan: Plan { jobs: vec![create_delivery_job(job_id.as_str(), (1., 0.))], ..create_empty_plan() },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };

    let result = check_e1104_no_reserved_ids(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem))).err();

    if let Some(action) = expected {
        assert_result("E1104", action, result);
    } else {
        assert!(result.is_none());
    }
}

#[test]
fn can_detect_empty_job() {
    let problem = Problem {
        plan: Plan { jobs: vec![Job { deliveries: Some(vec![]), ..create_job("job1") }], ..create_empty_plan() },
        ..create_empty_problem()
    };

    let result = check_e1105_empty_jobs(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem))).err();

    assert_result("E1105", "job1", result);
}

#[test]
fn can_detect_negative_duration() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_duration("job1", (1., 0.), -10.)], ..create_empty_plan() },
        ..create_empty_problem()
    };

    let result =
        check_e1106_negative_duration(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem))).err();

    assert_result("E1106", "job1", result);
}

#[test]
fn can_detect_negative_demand() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_demand("job1", (1., 0.), vec![0, -1])],
            ..create_empty_plan()
        },
        ..create_empty_problem()
    };

    let result = check_e1107_negative_demand(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem))).err();

    assert_result("E1107", "job1", result);
}
