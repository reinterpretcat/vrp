use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_two_breaks() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![99., 0.])],
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
                    breaks: Some(vec![
                        VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                            places: vec![VehicleBreakPlace {
                                duration: 2.0,
                                location: Some(vec![6., 0.].to_loc()),
                                tag: None,
                            }],
                            policy: None,
                        },
                        VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(100.), format_time(120.)]),
                            places: vec![VehicleBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy: None,
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
                cost: 412.,
                distance: 198,
                duration: 204,
                times: Timing { driving: 198, serving: 2, waiting: 0, break_time: 4 },
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
                    Stop {
                        location: vec![99., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:01:42Z".to_string(),
                            departure: "1970-01-01T00:01:45Z".to_string(),
                        },
                        distance: 99,
                        load: vec![0],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![99., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:01:42Z".to_string(),
                                    end: "1970-01-01T00:01:43Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            },
                            Activity {
                                job_id: "break".to_string(),
                                activity_type: "break".to_string(),
                                location: Some(vec![99., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:01:43Z".to_string(),
                                    end: "1970-01-01T00:01:45Z".to_string(),
                                }),
                                job_tag: None,
                                commute: None
                            }
                        ],
                    },
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:03:24Z", "1970-01-01T00:03:24Z"),
                        198,
                    )
                ],
                statistic: Statistic {
                    cost: 412.,
                    distance: 198,
                    duration: 204,
                    times: Timing { driving: 198, serving: 2, waiting: 0, break_time: 4 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
