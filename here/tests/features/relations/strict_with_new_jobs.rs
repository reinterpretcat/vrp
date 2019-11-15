use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_two_sequence_relations_with_two_vehicles_with_new_jobs() {
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
                create_delivery_job("job8", vec![8., 0.]),
                create_delivery_job("job9", vec![9., 0.]),
                create_delivery_job("job10", vec![10., 0.]),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job1", "job6", "job4", "job8"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job2", "job3", "job5", "job7"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                },
            ]),
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                places: create_default_vehicle_places(),
                capacity: vec![5],
                amount: 2,
                skills: None,
                limits: None,
                vehicle_break: None,
            }],
        },
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 114.,
                distance: 42,
                duration: 52,
                times: Timing { driving: 42, serving: 10, waiting: 0, break_time: 0 },
            },
            tours: vec![
                Tour {
                    vehicle_id: "my_vehicle_1".to_string(),
                    type_id: "my_vehicle".to_string(),
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            5,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (1., 0.),
                            4,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        ),
                        create_stop_with_activity(
                            "job6",
                            "delivery",
                            (6., 0.),
                            3,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        ),
                        create_stop_with_activity(
                            "job4",
                            "delivery",
                            (4., 0.),
                            2,
                            ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        ),
                        create_stop_with_activity(
                            "job8",
                            "delivery",
                            (8., 0.),
                            1,
                            ("1970-01-01T00:00:15Z", "1970-01-01T00:00:16Z"),
                        ),
                        create_stop_with_activity(
                            "job9",
                            "delivery",
                            (9., 0.),
                            0,
                            ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:27Z", "1970-01-01T00:00:27Z"),
                        )
                    ],
                    statistic: Statistic {
                        cost: 59.,
                        distance: 22,
                        duration: 27,
                        times: Timing { driving: 22, serving: 5, waiting: 0, break_time: 0 },
                    },
                },
                Tour {
                    vehicle_id: "my_vehicle_2".to_string(),
                    type_id: "my_vehicle".to_string(),
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            5,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        ),
                        create_stop_with_activity(
                            "job2",
                            "delivery",
                            (2., 0.),
                            4,
                            ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        ),
                        create_stop_with_activity(
                            "job3",
                            "delivery",
                            (3., 0.),
                            3,
                            ("1970-01-01T00:00:04Z", "1970-01-01T00:00:05Z"),
                        ),
                        create_stop_with_activity(
                            "job5",
                            "delivery",
                            (5., 0.),
                            2,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        ),
                        create_stop_with_activity(
                            "job7",
                            "delivery",
                            (7., 0.),
                            1,
                            ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        ),
                        create_stop_with_activity(
                            "job10",
                            "delivery",
                            (10., 0.),
                            0,
                            ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
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
                        cost: 55.,
                        distance: 20,
                        duration: 25,
                        times: Timing { driving: 20, serving: 5, waiting: 0, break_time: 0 },
                    },
                }
            ],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
