use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::prelude::Float;

#[test]
fn can_detect_invalid_break_time() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result =
        check_e1303_vehicle_breaks_time_is_correct(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1303".to_string()));
}

parameterized_test! {can_detect_zero_costs, (costs, expected), {
    can_detect_zero_costs_impl(costs, expected);
}}

can_detect_zero_costs! {
    case01: ((0.0001, 0.0001), None),
    case02: ((0., 0.0001), None),
    case03: ((0.0001, 0.), None),
    case04: ((0., 0.), Some("E1306".to_string())),
}

fn can_detect_zero_costs_impl(costs: (Float, Float), expected: Option<String>) {
    let (distance, time) = costs;
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: None, distance, time, span: None },
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result =
        check_e1306_vehicle_has_no_zero_costs(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
}

parameterized_test! {can_handle_rescheduling_with_required_break, (latest, expected), {
    can_handle_rescheduling_with_required_break_impl(latest, expected);
}}

can_handle_rescheduling_with_required_break! {
    case01: (None, None),
    case02: (Some(1.), None),
    case03: (Some(0.), None),
}

fn can_handle_rescheduling_with_required_break_impl(latest: Option<Float>, expected: Option<String>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: latest.map(format_time),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![VehicleBreak::Required {
                        time: VehicleRequiredBreakTime::OffsetTime { earliest: 10., latest: 10. },
                        duration: 2.,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = validate_vehicles(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    let error_code = result.err().and_then(|err| err.errors.first().map(|err| err.code.clone()));
    assert_eq!(error_code, expected);
}

parameterized_test! {can_handle_reload_resources, (resources, expected), {
    can_handle_reload_resources_impl(resources, expected);
}}

can_handle_reload_resources! {
    case01: (Some(vec!["r1"]), None),
    case02: (Some(vec!["r2"]), Some("E1308".to_string())),
    case03: (Some(vec!["r1", "r1"]), Some("E1308".to_string())),
}

fn can_handle_reload_resources_impl(resources: Option<Vec<&str>>, expected: Option<String>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    reloads: Some(vec![VehicleReload {
                        resource_id: Some("r1".to_string()),
                        ..create_default_reload()
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            resources: resources.map(|ids| {
                ids.iter().map(|id| VehicleResource::Reload { id: id.to_string(), capacity: vec![2] }).collect()
            }),
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result =
        check_e1308_vehicle_reload_resources(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
}

#[test]
fn can_detect_inverted_job_times() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    job_times: Some(JobTimeConstraints {
                        earliest_first: Some(format_time(200.)),
                        latest_last: Some(format_time(100.)),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1309_vehicle_job_times(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1309".to_string()));
}

parameterized_test! {can_reject_malformed_job_times, (earliest_first, latest_last), {
    can_reject_malformed_job_times_impl(earliest_first, latest_last);
}}

can_reject_malformed_job_times! {
    case01_malformed_earliest: (Some("08:00"), None),
    case02_malformed_latest: (None, Some("not a timestamp")),
    case03_both_malformed: (Some(""), Some("2026-13-45T99:00:00Z")),
}

fn can_reject_malformed_job_times_impl(earliest_first: Option<&str>, latest_last: Option<&str>) {
    // Validation is the first thing that reads the problem, so a bound that is not a timestamp
    // reaches this rule unchecked. It must come back as E1309, not as a panic in `parse_time`.
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    job_times: Some(JobTimeConstraints {
                        earliest_first: earliest_first.map(String::from),
                        latest_last: latest_last.map(String::from),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1309_vehicle_job_times(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1309".to_string()));
}

parameterized_test! {can_validate_job_times, (job_times, expected), {
    can_validate_job_times_impl(job_times, expected);
}}

can_validate_job_times! {
    case01_no_job_times: (None, None),
    case02_earliest_only: (Some((Some(100.), None)), None),
    case03_latest_only: (Some((None, Some(900.))), None),
    case04_both_at_shift_bounds: (Some((Some(0.), Some(1000.))), None),
    case05_equal_pair: (Some((Some(200.), Some(200.))), Some("E1309".to_string())),
    case06_earliest_before_shift_start: (Some((Some(-50.), None)), Some("E1309".to_string())),
    case07_latest_after_shift_end: (Some((None, Some(1050.))), Some("E1309".to_string())),
}

fn can_validate_job_times_impl(job_times: Option<(Option<Float>, Option<Float>)>, expected: Option<String>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    job_times: job_times.map(|(earliest, latest)| JobTimeConstraints {
                        earliest_first: earliest.map(format_time),
                        latest_last: latest.map(format_time),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1309_vehicle_job_times(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
}
