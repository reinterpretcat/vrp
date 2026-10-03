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
    case01: ((0.0001, 0.0001, None), None),
    case02: ((0., 0.0001, None), None),
    case03: ((0.0001, 0., None), None),
    case04: ((0., 0., None), Some("E1306".to_string())),
    case05_overtime_only_is_a_cost: ((0., 0., Some(0.02)), None),
    case06_zero_overtime_still_fails: ((0., 0., Some(0.)), Some("E1306".to_string())),
}

fn can_detect_zero_costs_impl(costs: (Float, Float, Option<Float>), expected: Option<String>) {
    let (distance, time, overtime) = costs;
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: None, distance, time, overtime, off_hours: None, span: None },
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

type WindowCase = (&'static str, (Float, Float), Option<&'static str>);

parameterized_test! {can_validate_visit_windows, (windows, tag, expected), {
    can_validate_visit_windows_impl(windows, tag, expected);
}}

can_validate_visit_windows! {
    case01_valid: (vec![("regular", (100., 400.), Some("working")), ("working", (0., 800.), None)], Some("regular"), None),
    case02_backwards: (vec![("regular", (400., 100.), None)], None, Some("E1310")),
    case03_empty: (vec![("regular", (100., 100.), None)], None, Some("E1310")),
    case04_outside_shift: (vec![("regular", (0., 2000.), None)], None, Some("E1310")),
    case05_unknown_fallback: (vec![("regular", (100., 400.), Some("nope"))], None, Some("E1310")),
    case06_fallback_cycle: (vec![("a", (100., 400.), Some("b")), ("b", (100., 400.), Some("a"))], None, Some("E1310")),
    case07_unknown_tag: (vec![("regular", (100., 400.), None)], Some("recurring"), Some("E1310")),
    case08_fallback_need_not_contain_its_window: (vec![("morning", (100., 200.), Some("evening")), ("evening", (300., 400.), None)], Some("morning"), None),
}

fn can_validate_visit_windows_impl(windows: Vec<WindowCase>, tag: Option<&str>, expected: Option<&str>) {
    let windows = windows
        .into_iter()
        .map(|(name, (earliest, latest), fallback)| {
            (
                name.to_string(),
                VisitWindowJson {
                    earliest: format_time(earliest),
                    latest: format_time(latest),
                    fallback: fallback.map(str::to_string),
                    bridge: false,
                },
            )
        })
        .collect();
    let problem = Problem {
        plan: Plan {
            jobs: vec![Job { visit_window: tag.map(str::to_string), ..create_delivery_job("job1", (1., 0.)) }],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift { visit_windows: Some(windows), ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1310_vehicle_visit_windows(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected.map(str::to_string));
}

#[test]
fn can_reject_a_malformed_visit_window_without_panicking() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    visit_windows: Some(std::collections::HashMap::from([(
                        "regular".to_string(),
                        VisitWindowJson {
                            earliest: "08:45".to_string(),
                            latest: format_time(400.),
                            fallback: None,
                            bridge: false,
                        },
                    )])),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1310_vehicle_visit_windows(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1310".to_string()));
}

parameterized_test! {can_validate_regular_hours, (hours, off_hours, expected), {
    can_validate_regular_hours_impl(hours, off_hours, expected);
}}

can_validate_regular_hours! {
    case01_valid: (Some((100., 400.)), Some(0.02), None),
    case02_backwards: (Some((400., 100.)), Some(0.02), Some("E1311")),
    case03_empty: (Some((100., 100.)), None, Some("E1311")),
    case04_outside_shift: (Some((0., 2000.)), Some(0.02), Some("E1311")),
    case05_rate_without_hours: (None, Some(0.02), Some("E1311")),
    case06_hours_without_rate: (Some((100., 400.)), None, None),
    case07_neither: (None, None, None),
}

fn can_validate_regular_hours_impl(hours: Option<(Float, Float)>, off_hours: Option<Float>, expected: Option<&str>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { off_hours, ..create_default_vehicle_costs() },
                shifts: vec![VehicleShift {
                    regular_hours: hours.map(|(earliest, latest)| RegularHoursJson {
                        earliest: format_time(earliest),
                        latest: format_time(latest),
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1311_vehicle_regular_hours(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected.map(str::to_string));
}

#[test]
fn can_reject_malformed_regular_hours_without_panicking() {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    regular_hours: Some(RegularHoursJson { earliest: "08:30".to_string(), latest: format_time(400.) }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result = check_e1311_vehicle_regular_hours(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1311".to_string()));
}
