use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_shift_start() -> ShiftStart {
    ShiftStart { earliest: format_time(0.), latest: Some(format_time(0.)), location: vec![0., 0.].to_loc() }
}

#[test]
fn can_assign_break_during_travel() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: create_shift_start(),
                    breaks: Some(vec![VehicleBreak::Required {
                        time: VehicleRequiredBreakTime::ExactTime(format_time(7.)),
                        duration: 2.,
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
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_duration("job1", vec![5., 0.], 3.)], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: create_shift_start(),
                    breaks: Some(vec![VehicleBreak::Required {
                        time: VehicleRequiredBreakTime::ExactTime(format_time(7.)),
                        duration: 2.,
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
                        location: vec![5., 0.].to_loc(),
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
                                location: Some(vec![5., 0.].to_loc()),
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
