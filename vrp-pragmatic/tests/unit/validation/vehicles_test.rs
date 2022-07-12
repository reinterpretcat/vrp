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
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result =
        check_e1303_vehicle_breaks_time_is_correct(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), Some("E1303".to_string()));
}

parameterized_test! {can_detect_invalid_area, (areas, area_ids, expected), {
    can_detect_invalid_area_impl(areas, area_ids, expected);
}}

can_detect_invalid_area! {
    case01: (None, None, None),
    case02: (Some(vec![("1", vec!["job1", "job2"])]), Some(vec!["1"]), None),
    case03: (Some(vec![("1", vec!["job1", "job2", "job2"])]), Some(vec!["1", "2"]), Some(())),
    case05: (Some(vec![("1", vec!["job1"]), ("2", vec!["job2"])]), Some(vec!["1", "2"]), None),
    case06: (Some(vec![("1", vec!["job1"]), ("2", vec!["job1"])]), Some(vec!["1"]), None),
    case07: (Some(vec![("1", vec!["job1", "job2"]), ("2", vec!["job2"])]), Some(vec!["1", "2"]), Some(())),
}

fn can_detect_invalid_area_impl(
    areas: Option<Vec<(&str, Vec<&str>)>>,
    area_ids: Option<Vec<&str>>,
    expected: Option<()>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (2., 0.))],
            areas: areas.as_ref().map(|areas| {
                areas
                    .iter()
                    .map(|(area_id, job_ids)| Area {
                        id: area_id.to_string(),
                        jobs: job_ids.iter().map(|job_id| job_id.to_string()).collect(),
                    })
                    .collect()
            }),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    tour_size: None,
                    areas: area_ids.map(|area_ids| {
                        vec![area_ids
                            .iter()
                            .map(|area_id| AreaLimit { area_id: area_id.to_string(), job_value: 1. })
                            .collect()]
                    }),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result =
        check_e1305_vehicle_limit_area_is_correct(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected.map(|_| "E1305".to_string()));
}

parameterized_test! {can_detect_invalid_dispatch, (allowed_areas, expected), {
    can_detect_invalid_dispatch_impl(allowed_areas, expected);
}}

can_detect_invalid_dispatch! {
    case01: (&[(0., (0., 10.))], None),
    case02: (&[(1., (0., 10.))], None),
    case03: (&[(1., (0., 10.)), (1., (0., 10.))], Some("E1306".to_string())),
    case04: (&[(1., (0., 10.)), (2., (0., 10.))], None),

    case05: (&[(1., (0., 10.))], None),
    case06: (&[(1., (1001., 1010.))], Some("E1306".to_string())),
    case07: (&[(1., (10., 1.))], Some("E1306".to_string())),
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
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result =
        check_e1306_vehicle_dispatch_is_correct(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
}

parameterized_test! {can_detect_zero_costs, (costs, expected), {
    can_detect_zero_costs_impl(costs, expected);
}}

can_detect_zero_costs! {
    case01: ((0.0001, 0.0001), None),
    case02: ((0., 0.0001), None),
    case03: ((0.0001, 0.), None),
    case04: ((0., 0.), Some("E1307".to_string())),
}

fn can_detect_zero_costs_impl(costs: (f64, f64), expected: Option<String>) {
    let (distance, time) = costs;
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: None, distance, time },
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result =
        check_e1307_vehicle_has_no_zero_costs(&ValidationContext::new(&problem, None, &CoordIndex::new(&problem)));

    assert_eq!(result.err().map(|err| err.code), expected);
}

parameterized_test! {can_handle_rescheduling_with_required_break, (latest, expected), {
    can_handle_rescheduling_with_required_break_impl(latest, expected);
}}

can_handle_rescheduling_with_required_break! {
    case01: (None, Some("E1308".to_string())),
    case02: (Some(1.), Some("E1308".to_string())),
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
                        time: VehicleRequiredBreakTime::OffsetTime(10.),
                        duration: 2.,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: vec![],
        },
        ..create_empty_problem()
    };

    let result = check_e1308_vehicle_required_break_rescheduling(&ValidationContext::new(
        &problem,
        None,
        &CoordIndex::new(&problem),
    ));

    assert_eq!(result.err().map(|err| err.code), expected);
}
