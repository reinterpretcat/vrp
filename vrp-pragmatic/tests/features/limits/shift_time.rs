use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

fn create_vehicle_type_with_shift_time_limit(shift_time: f64) -> VehicleType {
    VehicleType {
        limits: Some(VehicleLimits {
            max_distance: None,
            shift_time: Some(shift_time),
            tour_size: None,
            allowed_areas: None,
        }),
        ..create_default_vehicle_type()
    }
}

#[test]
fn can_limit_one_job_by_shift_time() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![100., 0.])], relations: Option::None },
        fleet: Fleet {
            vehicles: vec![create_vehicle_type_with_shift_time_limit(99.)],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1, 100, 100, 1],
        distances: vec![1, 1, 1, 1],
        error_codes: Option::None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 0.,
                distance: 0,
                duration: 0,
                times: Timing { driving: 0, serving: 0, waiting: 0, break_time: 0 },
            },
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: 102,
                    description: "cannot be assigned due to shift time constraint of vehicle".to_string()
                }]
            }]),
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_skip_job_from_multiple_because_of_shift_time() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", vec![1., 0.], 10.),
                create_delivery_job_with_duration("job2", vec![2., 0.], 10.),
                create_delivery_job_with_duration("job3", vec![3., 0.], 10.),
                create_delivery_job_with_duration("job4", vec![4., 0.], 10.),
                create_delivery_job_with_duration("job5", vec![5., 0.], 10.),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_type_with_shift_time_limit(40.)],
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
                distance: 6,
                duration: 36,
                times: Timing { driving: 6, serving: 30, waiting: 0, break_time: 0 },
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
                        3,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (3., 0.),
                        2,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:13Z"),
                        3
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:24Z"),
                        4
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:25Z", "1970-01-01T00:00:35Z"),
                        5
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:36Z", "1970-01-01T00:00:36Z"),
                        6
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 6,
                    duration: 36,
                    times: Timing { driving: 6, serving: 30, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: Some(vec![
                UnassignedJob {
                    job_id: "job4".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: 102,
                        description: "cannot be assigned due to shift time constraint of vehicle".to_string()
                    }]
                },
                UnassignedJob {
                    job_id: "job5".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: 102,
                        description: "cannot be assigned due to shift time constraint of vehicle".to_string()
                    }]
                }
            ]),
            ..create_empty_solution()
        }
    );
}

/*#[test]
fn can_serve_job_when_it_starts_late() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", vec![1., 0.], vec![(100, 200)], 10.)],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_type_with_shift_time_limit(50.)],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert!(!solution.tours.is_empty());
}
*/
