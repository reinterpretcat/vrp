use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_two_strict_relations_with_two_vehicles_with_new_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_delivery_job("job4", (4., 0.)),
                create_delivery_job("job5", (5., 0.)),
                create_delivery_job("job6", (6., 0.)),
                create_delivery_job("job7", (7., 0.)),
                create_delivery_job("job8", (8., 0.)),
                create_delivery_job("job9", (9., 0.)),
                create_delivery_job("job10", (10., 0.)),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job1", "job6", "job4", "job8"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job2", "job3", "job5", "job7"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
            ]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                capacity: vec![5],
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
                cost: 114.,
                distance: 42,
                duration: 52,
                times: Timing { driving: 42, serving: 10, ..Timing::default() },
            },
            tours: vec![
                Tour {
                    vehicle_id: "my_vehicle_1".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            5,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (1., 0.),
                            4,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            1
                        ),
                        create_stop_with_activity(
                            "job6",
                            "delivery",
                            (6., 0.),
                            3,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                            6
                        ),
                        create_stop_with_activity(
                            "job4",
                            "delivery",
                            (4., 0.),
                            2,
                            ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                            8
                        ),
                        create_stop_with_activity(
                            "job8",
                            "delivery",
                            (8., 0.),
                            1,
                            ("1970-01-01T00:00:15Z", "1970-01-01T00:00:16Z"),
                            12
                        ),
                        create_stop_with_activity(
                            "job9",
                            "delivery",
                            (9., 0.),
                            0,
                            ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                            13
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:27Z", "1970-01-01T00:00:27Z"),
                            22
                        )
                    ],
                    statistic: Statistic {
                        cost: 59.,
                        distance: 22,
                        duration: 27,
                        times: Timing { driving: 22, serving: 5, ..Timing::default() },
                    },
                },
                Tour {
                    vehicle_id: "my_vehicle_2".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            5,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0
                        ),
                        create_stop_with_activity(
                            "job2",
                            "delivery",
                            (2., 0.),
                            4,
                            ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                            2
                        ),
                        create_stop_with_activity(
                            "job3",
                            "delivery",
                            (3., 0.),
                            3,
                            ("1970-01-01T00:00:04Z", "1970-01-01T00:00:05Z"),
                            3
                        ),
                        create_stop_with_activity(
                            "job5",
                            "delivery",
                            (5., 0.),
                            2,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                            5
                        ),
                        create_stop_with_activity(
                            "job7",
                            "delivery",
                            (7., 0.),
                            1,
                            ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                            7
                        ),
                        create_stop_with_activity(
                            "job10",
                            "delivery",
                            (10., 0.),
                            0,
                            ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                            10
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:25Z", "1970-01-01T00:00:25Z"),
                            20
                        )
                    ],
                    statistic: Statistic {
                        cost: 55.,
                        distance: 20,
                        duration: 25,
                        times: Timing { driving: 20, serving: 5, ..Timing::default() },
                    },
                }
            ],
            ..create_empty_solution()
        }
    );
}
