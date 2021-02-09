use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_skip_break_when_vehicle_not_used() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
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
                        breaks: Some(vec![VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                            duration: 2.0,
                            locations: Some(vec![vec![6., 0.].to_loc()]),
                        }]),
                        reloads: None,
                    }],
                    ..create_default_vehicle_type()
                },
                create_default_vehicle("vehicle_without_break"),
            ],
            profiles: create_default_profiles(),
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
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
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
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_skip_break_when_jobs_completed() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_duration("job1", vec![1., 0.], 10.)],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                        duration: 2.0,
                        locations: Some(vec![vec![6., 0.].to_loc()]),
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
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
                times: Timing { driving: 2, serving: 10, waiting: 0, break_time: 0 },
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
                    times: Timing { driving: 2, serving: 10, waiting: 0, break_time: 0 },
                },
            }],
            violations: Some(vec![Violation::Break { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_skip_second_break_when_jobs_completed() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![
                        VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                            duration: 2.0,
                            locations: Some(vec![vec![6., 0.].to_loc()]),
                        },
                        VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(100.), format_time(120.)]),
                            duration: 2.0,
                            locations: None,
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
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
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 2 },
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
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
