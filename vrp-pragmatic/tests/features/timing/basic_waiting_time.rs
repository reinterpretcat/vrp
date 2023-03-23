use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::helpers::*;

#[test]
fn can_wait_for_job_start() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (1., 0.), vec![(0, 1)], 0.),
                create_delivery_job_with_times("job2", (2., 0.), vec![(10, 20)], 0.),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 4,
                duration: 12,
                times: Timing { driving: 4, serving: 0, waiting: 8, ..Timing::default() },
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
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:01Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:10Z"),
                        2
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                        4
                    )
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 4,
                    duration: 12,
                    times: Timing { driving: 4, serving: 0, waiting: 8, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_skip_initial_waiting() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(10, 20)], 10.)],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:09Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:20Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:21Z", "1970-01-01T00:00:21Z"),
                        2
                    )
                ],
                statistic: Statistic {
                    cost: 24.,
                    distance: 2,
                    duration: 12,
                    times: Timing { driving: 2, serving: 10, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_consider_latest_departure_time() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(10, 20)], 10.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T00:00:00Z".to_string(),
                        latest: Some("1970-01-01T00:00:05Z".to_string()),
                        location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                    },
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle("my_vehicle")
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 28.,
                distance: 2,
                duration: 16,
                times: Timing { driving: 2, serving: 10, waiting: 4, ..Timing::default() },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:05Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:06Z", "1970-01-01T00:00:20Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:21Z", "1970-01-01T00:00:21Z"),
                        2
                    )
                ],
                statistic: Statistic {
                    cost: 28.,
                    distance: 2,
                    duration: 16,
                    times: Timing { driving: 2, serving: 10, waiting: 4, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
