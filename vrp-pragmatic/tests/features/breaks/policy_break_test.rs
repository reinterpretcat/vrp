use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

parameterized_test! {can_skip_break_when_vehicle_not_used, policy, {
    can_skip_break_when_vehicle_not_used_impl(policy);
}}

can_skip_break_when_vehicle_not_used! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
    case_03: Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd),
}

fn can_skip_break_when_vehicle_not_used_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    shifts: vec![VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(0.),
                            latest: None,
                            location: vec![100., 0.].to_loc(),
                        },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(1000.).to_string(),
                            location: vec![100., 0.].to_loc(),
                        }),
                        dispatch: None,
                        breaks: Some(vec![VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                            places: vec![VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some(vec![6., 0.].to_loc()),
                                tag: None,
                            }],
                            policy,
                        }]),
                        reloads: None,
                    }],
                    ..create_default_vehicle_type()
                },
                create_default_vehicle("vehicle_without_break"),
            ],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 52.,
                distance: 20,
                duration: 22,
                times: Timing { driving: 20, serving: 2, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "vehicle_without_break_1".to_string(),
                type_id: "vehicle_without_break".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        10,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                        15,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z"),
                        20,
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 20,
                    duration: 22,
                    times: Timing { driving: 20, serving: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

parameterized_test! {can_skip_break_when_jobs_completed, policy, {
    can_skip_break_when_jobs_completed_impl(policy);
}}

can_skip_break_when_jobs_completed! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
    case_03: Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd),
}

fn can_skip_break_when_jobs_completed_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_duration("job1", vec![1., 0.], 10.)], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 2.0,
                            location: Some(vec![6., 0.].to_loc()),
                            tag: None,
                        }],
                        policy,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 24.,
                distance: 2,
                duration: 12,
                times: Timing { driving: 2, serving: 10, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:11Z"),
                        1,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                        2,
                    )
                ],
                statistic: Statistic {
                    cost: 24.,
                    distance: 2,
                    duration: 12,
                    times: Timing { driving: 2, serving: 10, ..Timing::default() },
                },
            }],
            violations: Some(vec![Violation::Break { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
            ..create_empty_solution()
        }
    );
}

parameterized_test! {can_skip_second_break_when_jobs_completed, policy, {
    can_skip_second_break_when_jobs_completed_impl(policy);
}}

can_skip_second_break_when_jobs_completed! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
}

fn can_skip_second_break_when_jobs_completed_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                            places: vec![VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some(vec![6., 0.].to_loc()),
                                tag: None,
                            }],
                            policy: policy.clone(),
                        },
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(100.), format_time(120.)]),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy,
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 54.,
                distance: 20,
                duration: 24,
                times: Timing { driving: 20, serving: 2, break_time: 2, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        1,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        5,
                    ),
                    create_stop_with_activity(
                        "break",
                        "break",
                        (6., 0.),
                        1,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:09Z"),
                        6,
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:24Z", "1970-01-01T00:00:24Z"),
                        20,
                    )
                ],
                statistic: Statistic {
                    cost: 54.,
                    distance: 20,
                    duration: 24,
                    times: Timing { driving: 20, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

parameterized_test! {can_skip_break_depending_on_policy, (policy, location, time, expected), {
    can_skip_break_depending_on_policy_impl(policy, location, time, expected);
}}

can_skip_break_depending_on_policy! {
    case_01: (Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd), 5., (5., 11.), 0),
    case_02: (Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd), 5., (5., 8.), 2),

    case_03: (Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection), 5., (5., 11.), 2),
    case_04: (Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection), 5., (5., 8.), 2),
}

fn can_skip_break_depending_on_policy_impl(
    policy: Option<VehicleOptionalBreakPolicy>,
    location: f64,
    time: (f64, f64),
    expected: i64,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_duration("job1", vec![location, 0.], 0.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(time.0), format_time(time.1)]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert!(solution.violations.is_none());
    assert_eq!(solution.statistic.times.break_time, expected);
}
