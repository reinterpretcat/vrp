use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_one_pickup_delivery_job_with_one_vehicle() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_pickup_delivery_job("job1", (1., 0.), (2., 0.))], ..create_empty_plan() },
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
                            .load(vec![0])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1, 2)
                            .load(vec![1])
                            .distance(1)
                            .build_single_tag("job1", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3, 4)
                            .load(vec![0])
                            .distance(2)
                            .build_single_tag("job1", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(6, 6)
                            .load(vec![0])
                            .distance(4)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(4).serving(2).build())
                    .build()
            )
            .build()
    );
}
