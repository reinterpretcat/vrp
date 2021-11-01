use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_assign_multi_and_single_job_as_pickups_specified() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple", vec![1., 0.]),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((6., 0.), 1., vec![2])],
                ),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
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
                cost: 46.,
                distance: 16,
                duration: 20,
                times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
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
                        0
                    ),
                    create_stop_with_activity(
                        "simple",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2,
                        "p1"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (8., 0.),
                        2,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        8,
                        "p2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10,
                        "d1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:20Z"),
                        16
                    )
                ],
                statistic: Statistic {
                    cost: 46.,
                    distance: 16,
                    duration: 20,
                    times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_assign_multi_job_in_pickup_effective_way() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((4., 0.), 1., vec![1]), ((2., 0.), 1., vec![1])],
                vec![((6., 0.), 1., vec![2])],
            )],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
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
                cost: 37.,
                distance: 12,
                duration: 15,
                times: Timing { driving: 12, serving: 3, waiting: 0, break_time: 0 },
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
                        "multi",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        2,
                        "p2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (4., 0.),
                        2,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        4,
                        "p1"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:08Z", "1970-01-01T00:00:09Z"),
                        6,
                        "d1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:15Z", "1970-01-01T00:00:15Z"),
                        12
                    )
                ],
                statistic: Statistic {
                    cost: 37.,
                    distance: 12,
                    duration: 15,
                    times: Timing { driving: 12, serving: 3, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
