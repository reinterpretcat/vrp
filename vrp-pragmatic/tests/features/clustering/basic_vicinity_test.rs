use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_cluster_simple_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_delivery_job("job4", vec![10., 0.]),
            ],
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    moving_duration: 3.,
                    moving_distance: 3.,
                    min_shared_time: None,
                    smallest_time_window: None,
                    max_jobs_per_cluster: None,
                },
                visiting: VicinityVisitPolicy::ClosedContinuation,
                serving: VicinityServingPolicy::Original,
                filtering: None,
            }),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_default_vehicle_type()
            }],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 38.,
                distance: 10,
                duration: 18,
                times: Timing { driving: 10, serving: 4, waiting: 0, break_time: 0 },
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
                        4,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    Stop {
                        location: vec![3., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:03Z".to_string(),
                            departure: "1970-01-01T00:00:10Z".to_string(),
                        },
                        distance: 3,
                        load: vec![1],
                        activities: vec![
                            Activity {
                                job_id: "job3".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![3., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:03Z".to_string(),
                                    end: "1970-01-01T00:00:04Z".to_string(),
                                }),
                                job_tag: None,
                                commute: Some(Commute { forward: None, backward: None }),
                            },
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![2., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:05Z".to_string(),
                                    end: "1970-01-01T00:00:06Z".to_string(),
                                }),
                                job_tag: None,
                                commute: Some(Commute {
                                    forward: Some(CommuteInfo {
                                        distance: 1.,
                                        time: Interval {
                                            start: "1970-01-01T00:00:04Z".to_string(),
                                            end: "1970-01-01T00:00:05Z".to_string()
                                        }
                                    }),
                                    backward: None,
                                }),
                            },
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![1., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:07Z".to_string(),
                                    end: "1970-01-01T00:00:08Z".to_string(),
                                }),
                                job_tag: None,
                                commute: Some(Commute {
                                    forward: Some(CommuteInfo {
                                        distance: 1.,
                                        time: Interval {
                                            start: "1970-01-01T00:00:06Z".to_string(),
                                            end: "1970-01-01T00:00:07Z".to_string()
                                        }
                                    }),
                                    backward: Some(CommuteInfo {
                                        distance: 2.,
                                        time: Interval {
                                            start: "1970-01-01T00:00:08Z".to_string(),
                                            end: "1970-01-01T00:00:10Z".to_string()
                                        }
                                    })
                                }),
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                        10,
                    ),
                ],
                statistic: Statistic {
                    cost: 38.,
                    distance: 10,
                    duration: 18,
                    times: Timing { driving: 10, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
