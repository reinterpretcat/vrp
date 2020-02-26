use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_create_solution() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
        },
        fleet: Fleet { types: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_heuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 52.,
                distance: 20,
                duration: 22,
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z")
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z")
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z")
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z")
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 20,
                    duration: 22,
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}

#[test]
fn can_merge_activities_in_one_stop() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![5., 0.])],
            relations: Option::None,
        },
        fleet: Fleet { types: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_heuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 32.,
                distance: 10,
                duration: 12,
                times: Timing { driving: 10, serving: 2, waiting: 0, break_time: 0 },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z")
                    ),
                    Stop {
                        location: vec![5., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:05Z".to_string(),
                            departure: "1970-01-01T00:00:07Z".to_string()
                        },
                        load: vec![0],
                        activities: vec![
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![5., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:05Z".to_string(),
                                    end: "1970-01-01T00:00:06Z".to_string()
                                }),
                                job_tag: None
                            },
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![5., 0.].to_loc()),
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:06Z".to_string(),
                                    end: "1970-01-01T00:00:07Z".to_string()
                                }),
                                job_tag: None
                            }
                        ]
                    },
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z")
                    )
                ],
                statistic: Statistic {
                    cost: 32.,
                    distance: 10,
                    duration: 12,
                    times: Timing { driving: 10, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
