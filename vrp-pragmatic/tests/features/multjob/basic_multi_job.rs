use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_assign_multi_and_single_job_as_pickups_specified() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple", (1., 0.)),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1, vec![1]), ((8., 0.), 1, vec![1])],
                    vec![((6., 0.), 1, vec![2])],
                ),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])], ..create_default_fleet() },
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
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1, 2)
                            .load(vec![0])
                            .distance(1)
                            .build_single("simple", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3, 4)
                            .load(vec![1])
                            .distance(2)
                            .build_single_tag("multi", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(10, 11)
                            .load(vec![2])
                            .distance(8)
                            .build_single_tag("multi", "pickup", "p2"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(13, 14)
                            .load(vec![0])
                            .distance(10)
                            .build_single_tag("multi", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(20, 20)
                            .load(vec![0])
                            .distance(16)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(16).serving(4).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_assign_multi_job_in_pickup_effective_way() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((4., 0.), 1, vec![1]), ((2., 0.), 1, vec![1])],
                vec![((6., 0.), 1, vec![2])],
            )],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])], ..create_default_fleet() },
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
                            .load(vec![0])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2, 3)
                            .load(vec![1])
                            .distance(2)
                            .build_single_tag("multi", "pickup", "p2"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(5, 6)
                            .load(vec![2])
                            .distance(4)
                            .build_single_tag("multi", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(8, 9)
                            .load(vec![0])
                            .distance(6)
                            .build_single_tag("multi", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(15, 15)
                            .load(vec![0])
                            .distance(12)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(12).serving(3).build())
                    .build()
            )
            .build()
    );
}
