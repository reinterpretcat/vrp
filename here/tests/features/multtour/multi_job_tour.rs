use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple", vec![1., 0.]),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((6., 0.), 1., vec![2])],
                ),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0), location: vec![0., 0.] },
                    end: Some(VehiclePlace { time: format_time(100).to_string(), location: vec![0., 0.] }),
                    breaks: None,
                    max_tours: Some(2),
                    load_time: Some(2),
                }],
                capacity: vec![2],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_heuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 46.,
                distance: 16,
                duration: 20,
                times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity(
                        "simple",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        "1"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (8., 0.),
                        2,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        "2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        "1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:20Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 46.,
                    distance: 16,
                    duration: 20,
                    times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}

#[test]
fn can_assign_multi_job_in_pickup_effective_way() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((4., 0.), 1., vec![1]), ((2., 0.), 1., vec![1])],
                vec![((6., 0.), 1., vec![2])],
            )],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
            profiles: create_default_profiles(),
        },
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 37.,
                distance: 12,
                duration: 15,
                times: Timing { driving: 12, serving: 3, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        "2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (4., 0.),
                        2,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        "1"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:08Z", "1970-01-01T00:00:09Z"),
                        "1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:15Z", "1970-01-01T00:00:15Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 37.,
                    distance: 12,
                    duration: 15,
                    times: Timing { driving: 12, serving: 3, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
