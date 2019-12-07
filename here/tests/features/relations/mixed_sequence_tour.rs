use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_sequence_and_tour_relation_for_one_vehicle() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_delivery_job("job4", vec![4., 0.]),
                create_delivery_job("job5", vec![5., 0.]),
                create_delivery_job("job6", vec![6., 0.]),
                create_delivery_job("job7", vec![7., 0.]),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job4", "job2", "job6"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                },
                Relation {
                    type_field: RelationType::Tour,
                    jobs: to_strings(vec!["job1", "job3"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                },
            ]),
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![create_default_vehicle_shift()],
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: None,
            }],
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
                cost: 53.,
                distance: 18,
                duration: 25,
                times: Timing { driving: 18, serving: 7, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        7,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (4., 0.),
                        6,
                        ("1970-01-01T00:00:04Z", "1970-01-01T00:00:05Z"),
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        5,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                    ),
                    create_stop_with_activity(
                        "job6",
                        "delivery",
                        (6., 0.),
                        4,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:13Z"),
                    ),
                    create_stop_with_activity(
                        "job7",
                        "delivery",
                        (7., 0.),
                        3,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (5., 0.),
                        2,
                        ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:21Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:23Z", "1970-01-01T00:00:24Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:25Z", "1970-01-01T00:00:25Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 53.,
                    distance: 18,
                    duration: 25,
                    times: Timing { driving: 18, serving: 7, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
