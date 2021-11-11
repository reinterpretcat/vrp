use crate::format::problem::Objective::{MinimizeCost, MinimizeUnassignedJobs};
use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_test_objectives() -> Option<Vec<Vec<Objective>>> {
    Some(vec![vec![MinimizeUnassignedJobs { breaks: Some(10.) }], vec![MinimizeCost]])
}

#[test]
fn can_assign_interval_break_between_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![15., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeOffset(vec![5., 10.]),
                        places: vec![VehicleBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        objectives: create_test_objectives(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 74.,
                distance: 30,
                duration: 34,
                times: Timing { driving: 30, serving: 2, break_time: 2, ..Timing::default() },
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
                        0
                    ),
                    Stop {
                        location: vec![5., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:05Z".to_string(),
                            departure: "1970-01-01T00:00:08Z".to_string(),
                        },
                        distance: 5,
                        load: vec![1],
                        parking: None,
                        activities: vec![
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![5., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:05Z".to_string(),
                                    end: "1970-01-01T00:00:06Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            },
                            Activity {
                                job_id: "break".to_string(),
                                activity_type: "break".to_string(),
                                location: Some(vec![5., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:06Z".to_string(),
                                    end: "1970-01-01T00:00:08Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            }
                        ],
                    },
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (15., 0.),
                        0,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:19Z"),
                        15
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:34Z", "1970-01-01T00:00:34Z"),
                        30
                    )
                ],
                statistic: Statistic {
                    cost: 74.,
                    distance: 30,
                    duration: 34,
                    times: Timing { driving: 30, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_assign_interval_break_with_reload() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![10., 0.]),
                create_delivery_job("job2", vec![15., 0.]),
                create_delivery_job("job3", vec![20., 0.]),
                create_delivery_job("job4", vec![25., 0.]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(0.)),
                        location: vec![0., 0.].to_loc(),
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(1000.).to_string(),
                        location: vec![30., 0.].to_loc(),
                    }),
                    dispatch: None,
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeOffset(vec![8., 12.]),
                        places: vec![VehicleBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    reloads: Some(vec![VehicleReload {
                        times: Some(vec![vec![format_time(0.), format_time(1000.)]]),
                        location: vec![0., 0.].to_loc(),
                        duration: 3.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        objectives: create_test_objectives(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 139.,
                distance: 60,
                duration: 69,
                times: Timing { driving: 60, serving: 7, break_time: 2, ..Timing::default() },
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
                        0
                    ),
                    Stop {
                        location: vec![10., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:10Z".to_string(),
                            departure: "1970-01-01T00:00:13Z".to_string(),
                        },
                        distance: 10,
                        load: vec![1],
                        parking: None,
                        activities: vec![
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![10., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:10Z".to_string(),
                                    end: "1970-01-01T00:00:11Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            },
                            Activity {
                                job_id: "break".to_string(),
                                activity_type: "break".to_string(),
                                location: Some(vec![10., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:11Z".to_string(),
                                    end: "1970-01-01T00:00:13Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            }
                        ],
                    },
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (15., 0.),
                        0,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:19Z"),
                        15
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:34Z", "1970-01-01T00:00:37Z"),
                        30
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (20., 0.),
                        1,
                        ("1970-01-01T00:00:57Z", "1970-01-01T00:00:58Z"),
                        50
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (25., 0.),
                        0,
                        ("1970-01-01T00:01:03Z", "1970-01-01T00:01:04Z"),
                        55
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (30., 0.),
                        0,
                        ("1970-01-01T00:01:09Z", "1970-01-01T00:01:09Z"),
                        60
                    )
                ],
                statistic: Statistic {
                    cost: 139.,
                    distance: 60,
                    duration: 69,
                    times: Timing { driving: 60, serving: 7, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_consider_departure_rescheduling() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![5., 0.], vec![(10, 10)], 1.),
                create_delivery_job_with_times("job2", vec![10., 0.], vec![(10, 30)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeOffset(vec![10., 12.]),
                        places: vec![VehicleBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        objectives: create_test_objectives(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 1000);

    assert!(solution.violations.is_none());
    assert!(solution.unassigned.is_none());
}
