use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_strict_and_any_relation_for_one_vehicle() {
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
                    type_field: RelationType::Any,
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
                            .schedule_stamp(0., 0.)
                            .load(vec![7])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(4., 5.)
                            .load(vec![6])
                            .distance(4)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![5])
                            .distance(6)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(12., 13.)
                            .load(vec![4])
                            .distance(10)
                            .build_single("job6", "delivery"),
                        StopBuilder::default()
                            .coordinate((7., 0.))
                            .schedule_stamp(14., 15.)
                            .load(vec![3])
                            .distance(11)
                            .build_single("job7", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(17., 18.)
                            .load(vec![2])
                            .distance(13)
                            .build_single("job5", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(20., 21.)
                            .load(vec![1])
                            .distance(15)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(23., 24.)
                            .load(vec![0])
                            .distance(17)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(25., 25.)
                            .load(vec![0])
                            .distance(18)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(18).serving(7).build())
                    .build()
            )
            .build()
    );
}
