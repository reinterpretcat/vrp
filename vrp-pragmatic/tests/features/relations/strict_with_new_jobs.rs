use crate::format::problem::*;
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
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![5])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![4])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![3])
                            .distance(6)
                            .build_single("job6", "delivery"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![2])
                            .distance(8)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(15., 16.)
                            .load(vec![1])
                            .distance(12)
                            .build_single("job8", "delivery"),
                        StopBuilder::default()
                            .coordinate((9., 0.))
                            .schedule_stamp(17., 18.)
                            .load(vec![0])
                            .distance(13)
                            .build_single("job8", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(27., 27.)
                            .load(vec![0])
                            .distance(22)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(22).serving(5).build())
                    .build()
            )
            .tour(
                TourBuilder::default()
                    .vehicle_id("my_vehicle_2")
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![5])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2., 3.)
                            .load(vec![4])
                            .distance(2)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(4., 5.)
                            .load(vec![3])
                            .distance(3)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![2])
                            .distance(5)
                            .build_single("job5", "delivery"),
                        StopBuilder::default()
                            .coordinate((7., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![1])
                            .distance(7)
                            .build_single("job7", "delivery"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(14., 15.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job10", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(25., 25.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(5).build())
                    .build()
            )
            .build()
    );
}
