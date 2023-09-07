use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_one_pickup_delivery_and_two_deliveries_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_pickup_delivery_job("job2", (2., 0.), (3., 0.)),
                create_delivery_job("job3", (4., 0.)),
            ],
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
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2., 3.)
                            .load(vec![3])
                            .distance(2)
                            .build_single_tag("job2", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![2])
                            .distance(4)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![1])
                            .distance(5)
                            .build_single_tag("job2", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![0])
                            .distance(7)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(12., 12.)
                            .load(vec![0])
                            .distance(8)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(8).serving(4).build())
                    .build()
            )
            .build()
    );
}
