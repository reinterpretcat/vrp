use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;

fn create_test_problem() -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_index("job1", 0), create_delivery_job_with_index("job2", 1)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: None,
                        location: Location::Reference { index: 2 },
                    },
                    ..create_default_open_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

fn create_test_matrix() -> Matrix {
    Matrix {
        profile: Some("car".to_string()),
        timestamp: None,
        travel_times: vec![0, 3, 3, 1, 0, 3, 3, 2, 0],
        distances: vec![0, 3, 3, 1, 0, 3, 3, 2, 0],
        error_codes: None,
    }
}

#[test]
fn can_use_location_index() {
    let problem = create_test_problem();
    let matrix = create_test_matrix();

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 18.,
                distance: 3,
                duration: 5,
                times: Timing { driving: 3, serving: 2, ..Timing::default() }
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    Stop::Point(PointStop {
                        location: Location::Reference { index: 2 },
                        ..create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            2,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0
                        )
                        .to_point()
                    }),
                    Stop::Point(PointStop {
                        location: Location::Reference { index: 1 },
                        ..create_stop_with_activity(
                            "job2",
                            "delivery",
                            (0., 0.),
                            1,
                            ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                            2
                        )
                        .to_point()
                    }),
                    Stop::Point(PointStop {
                        location: Location::Reference { index: 0 },
                        ..create_stop_with_activity(
                            "job1",
                            "delivery",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:04Z", "1970-01-01T00:00:05Z"),
                            3
                        )
                        .to_point()
                    })
                ],
                statistic: Statistic {
                    cost: 18.,
                    distance: 3,
                    duration: 5,
                    times: Timing { driving: 3, serving: 2, ..Timing::default() }
                }
            }],
            unassigned: None,
            violations: None,
            extras: None
        }
    );
}
