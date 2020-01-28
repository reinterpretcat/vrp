use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_assign_interval_break_between_jobs() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![15., 0.])],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![create_default_vehicle_shift_with_breaks(vec![VehicleBreak {
                    times: VehicleBreakTime::IntervalWindow(vec![5., 10.]),
                    duration: 2.0,
                    location: None,
                }])],
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 74.,
                distance: 30,
                duration: 34,
                times: Timing { driving: 30, serving: 2, waiting: 0, break_time: 2 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    Stop {
                        location: vec![5., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:05Z".to_string(),
                            departure: "1970-01-01T00:00:08Z".to_string(),
                        },
                        load: vec![1],
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
                            }
                        ],
                    },
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (15., 0.),
                        0,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:19Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:34Z", "1970-01-01T00:00:34Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 74.,
                    distance: 30,
                    duration: 34,
                    times: Timing { driving: 30, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}

#[test]
fn can_assign_interval_break_with_reload() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![10., 0.]),
                create_delivery_job("job2", vec![15., 0.]),
                create_delivery_job("job3", vec![20., 0.]),
                create_delivery_job("job4", vec![25., 0.]),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(1000).to_string(), location: vec![30., 0.].to_loc() }),
                    breaks: Some(vec![VehicleBreak {
                        times: VehicleBreakTime::IntervalWindow(vec![8., 12.]),
                        duration: 2.0,
                        location: None,
                    }]),
                    reloads: Some(vec![VehicleReload {
                        times: Some(vec![vec![format_time(0), format_time(1000)]]),
                        location: vec![0., 0.].to_loc(),
                        duration: 3.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![2],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 139.,
                distance: 60,
                duration: 69,
                times: Timing { driving: 60, serving: 7, waiting: 0, break_time: 2 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    Stop {
                        location: vec![10., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:10Z".to_string(),
                            departure: "1970-01-01T00:00:13Z".to_string(),
                        },
                        load: vec![1],
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
                            }
                        ],
                    },
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (15., 0.),
                        0,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:19Z"),
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:34Z", "1970-01-01T00:00:37Z"),
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (20., 0.),
                        1,
                        ("1970-01-01T00:00:57Z", "1970-01-01T00:00:58Z"),
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (25., 0.),
                        0,
                        ("1970-01-01T00:01:03Z", "1970-01-01T00:01:04Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (30., 0.),
                        0,
                        ("1970-01-01T00:01:09Z", "1970-01-01T00:01:09Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 139.,
                    distance: 60,
                    duration: 69,
                    times: Timing { driving: 60, serving: 7, waiting: 0, break_time: 2 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
