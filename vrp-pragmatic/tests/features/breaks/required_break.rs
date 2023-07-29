use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_shift_start() -> ShiftStart {
    ShiftStart { earliest: format_time(0.), latest: Some(format_time(0.)), location: (0., 0.).to_loc() }
}

fn create_problem(jobs: Vec<Job>, vehicle_break: VehicleBreak, is_open: bool) -> Problem {
    let vehicle_shift = if is_open { create_default_open_vehicle_shift() } else { create_default_vehicle_shift() };
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: create_shift_start(),
                    breaks: Some(vec![vehicle_break]),
                    ..vehicle_shift
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

#[test]
fn can_assign_break_during_travel() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
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
                    Stop::Transit(TransitStop {
                        time: Schedule {
                            arrival: "1970-01-01T00:00:07Z".to_string(),
                            departure: "1970-01-01T00:00:09Z".to_string(),
                        },
                        load: vec![1],
                        activities: vec![Activity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                            commute: None
                        }],
                    }),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:24Z", "1970-01-01T00:00:24Z"),
                        20
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

#[test]
fn can_assign_break_during_activity() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job_with_duration("job1", (5., 0.), 3.)],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 35.,
                distance: 10,
                duration: 15,
                times: Timing { driving: 10, serving: 3, break_time: 2, ..Timing::default() },
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
                    Stop::Point(PointStop {
                        location: (5., 0.).to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:05Z".to_string(),
                            departure: "1970-01-01T00:00:10Z".to_string(),
                        },
                        distance: 5,
                        parking: None,
                        load: vec![0],
                        activities: vec![
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some((5., 0.).to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:05Z".to_string(),
                                    end: "1970-01-01T00:00:10Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            },
                            Activity {
                                job_id: "break".to_string(),
                                activity_type: "break".to_string(),
                                location: None,
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:07Z".to_string(),
                                    end: "1970-01-01T00:00:09Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            }
                        ],
                    }),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:15Z", "1970-01-01T00:00:15Z"),
                        10
                    )
                ],
                statistic: Statistic {
                    cost: 35.,
                    distance: 10,
                    duration: 15,
                    times: Timing { driving: 10, serving: 3, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_handle_required_break_when_its_start_falls_at_activity_end() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(6.), latest: format_time(6.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 34.,
                distance: 10,
                duration: 14,
                times: Timing { driving: 10, serving: 2, break_time: 2, ..Timing::default() },
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
                    Stop::Transit(TransitStop {
                        time: Schedule {
                            arrival: "1970-01-01T00:00:06Z".to_string(),
                            departure: "1970-01-01T00:00:08Z".to_string(),
                        },
                        load: vec![1],
                        activities: vec![Activity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                            commute: None
                        }],
                    }),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10
                    )
                ],
                statistic: Statistic {
                    cost: 34.,
                    distance: 10,
                    duration: 14,
                    times: Timing { driving: 10, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_skip_break_if_it_is_after_start_before_end_range() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(5.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(get_ids_from_tour(&solution.tours[0]).iter().flatten().all(|id| id != "break"));
}

// TODO check exact and offset use cases
#[test]
#[ignore]
fn can_reschedule_break_early_from_transport_to_activity() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(5.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 34.,
                distance: 10,
                duration: 14,
                times: Timing { driving: 10, serving: 2, break_time: 2, ..Timing::default() },
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
                    Stop::Point(PointStop {
                        location: (5., 0.).to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:05Z".to_string(),
                            departure: "1970-01-01T00:00:05Z".to_string(),
                        },
                        distance: 5,
                        parking: None,
                        load: vec![1],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some((5., 0.).to_loc()),
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
                                location: Some((5., 0.).to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:06Z".to_string(),
                                    end: "1970-01-01T00:00:08Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            }
                        ],
                    }),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10
                    )
                ],
                statistic: Statistic {
                    cost: 34.,
                    distance: 10,
                    duration: 14,
                    times: Timing { driving: 10, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
