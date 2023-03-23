use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_only_deliveries_as_static_demand() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job("job1", vec![], vec![((8., 0.), 2., vec![1]), ((2., 0.), 1., vec![1])])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_default_vehicle_type()
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
                cost: 29.,
                distance: 8,
                duration: 11,
                times: Timing { driving: 8, serving: 3, ..Timing::default() },
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
                    create_stop_with_activity_with_tag(
                        "job1",
                        "delivery",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        2,
                        "d2"
                    ),
                    create_stop_with_activity_with_tag(
                        "job1",
                        "delivery",
                        (8., 0.),
                        0,
                        ("1970-01-01T00:00:09Z", "1970-01-01T00:00:11Z"),
                        8,
                        "d1"
                    )
                ],
                statistic: Statistic {
                    cost: 29.,
                    distance: 8,
                    duration: 11,
                    times: Timing { driving: 8, serving: 3, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_use_only_pickups_as_static_demand() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job("job1", vec![((8., 0.), 2., vec![1]), ((2., 0.), 1., vec![1])], vec![])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (10., 0.).to_loc() }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
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
                cost: 33.,
                distance: 10,
                duration: 13,
                times: Timing { driving: 10, serving: 3, ..Timing::default() },
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
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity_with_tag(
                        "job1",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        2,
                        "p2"
                    ),
                    create_stop_with_activity_with_tag(
                        "job1",
                        "pickup",
                        (8., 0.),
                        2,
                        ("1970-01-01T00:00:09Z", "1970-01-01T00:00:11Z"),
                        8,
                        "p1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:13Z"),
                        10
                    ),
                ],
                statistic: Statistic {
                    cost: 33.,
                    distance: 10,
                    duration: 13,
                    times: Timing { driving: 10, serving: 3, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
