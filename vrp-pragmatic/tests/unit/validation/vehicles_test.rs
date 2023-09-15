use super::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;

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

parameterized_test! {can_detect_invalid_dispatch, (allowed_areas, expected), {
    can_detect_invalid_dispatch_impl(allowed_areas, expected);
}}

can_detect_invalid_dispatch! {
    case01: (&[(0., (0., 10.))], None),
    case02: (&[(1., (0., 10.))], None),
    case03: (&[(1., (0., 10.)), (1., (0., 10.))], Some("E1305".to_string())),
    case04: (&[(1., (0., 10.)), (2., (0., 10.))], None),

    case05: (&[(1., (0., 10.))], None),
    case06: (&[(1., (1001., 1010.))], Some("E1305".to_string())),
    case07: (&[(1., (10., 1.))], Some("E1305".to_string())),
}

fn can_detect_invalid_dispatch_impl(dispatch: &[(f64, (f64, f64))], expected: Option<String>) {
    let dispatch = Some(
        dispatch
            .iter()
            .cloned()
            .map(|(lat, times)| VehicleDispatch {
                location: Location::Coordinate { lat, lng: 0. },
                limits: vec![VehicleDispatchLimit { max: 1, start: format_time(times.0), end: format_time(times.1) }],
                tag: None,
            })
            .collect(),
    );
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift { dispatch, ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let result =
        check_e1305_vehicle_dispatch_is_correct(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
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

fn can_detect_zero_costs_impl(costs: (f64, f64), expected: Option<String>) {
    let (distance, time) = costs;
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: None, distance, time },
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
    case01: (None, Some("E1307".to_string())),
    case02: (Some(1.), Some("E1307".to_string())),
    case03: (Some(0.), None),
}

fn can_handle_rescheduling_with_required_break_impl(latest: Option<f64>, expected: Option<String>) {
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

    let result = check_e1307_vehicle_offset_break_rescheduling(&ValidationContext::new(
        &problem,
        None,
        &CoordIndex::new(&problem),
    ));

    assert_eq!(result.err().map(|err| err.code), expected);
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
