use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_strict_and_sequence_relation_for_one_vehicle() {
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
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job4", "job2", "job6"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["job1", "job3"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
            ]),
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0, 0)
                            .load(vec![7])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(4, 5)
                            .load(vec![6])
                            .distance(4)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7, 8)
                            .load(vec![5])
                            .distance(6)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(12, 13)
                            .load(vec![4])
                            .distance(10)
                            .build_single("job6", "delivery"),
                        StopBuilder::default()
                            .coordinate((7., 0.))
                            .schedule_stamp(14, 15)
                            .load(vec![3])
                            .distance(11)
                            .build_single("job7", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(17, 18)
                            .load(vec![2])
                            .distance(13)
                            .build_single("job5", "delivery"),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(22, 23)
                            .load(vec![1])
                            .distance(17)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(25, 26)
                            .load(vec![0])
                            .distance(19)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(29, 29)
                            .load(vec![0])
                            .distance(22)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(22).serving(7).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_use_strict_and_sequence_relation_for_two_vehicles() {
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
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job1", "job6"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["job3", "job7"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Strict,
                    jobs: to_strings(vec!["departure", "job2", "job8"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["job4", "job5"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
            ]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
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
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0, 0)
                            .load(vec![4])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1, 2)
                            .load(vec![3])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(7, 8)
                            .load(vec![2])
                            .distance(6)
                            .build_single("job6", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(11, 12)
                            .load(vec![1])
                            .distance(9)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((7., 0.))
                            .schedule_stamp(16, 17)
                            .load(vec![0])
                            .distance(13)
                            .build_single("job7", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(13).serving(4).build())
                    .build()
            )
            .tour(
                TourBuilder::default()
                    .vehicle_id("my_vehicle_2")
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0, 0)
                            .load(vec![4])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2, 3)
                            .load(vec![3])
                            .distance(2)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(9, 10)
                            .load(vec![2])
                            .distance(8)
                            .build_single("job8", "delivery"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(14, 15)
                            .load(vec![1])
                            .distance(12)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(16, 17)
                            .load(vec![0])
                            .distance(13)
                            .build_single("job5", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(13).serving(4).build())
                    .build()
            )
            .build()
    );
}
