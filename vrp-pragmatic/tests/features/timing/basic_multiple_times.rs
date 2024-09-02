use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_multiple_times() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 10)], 0),
                create_delivery_job_with_times("job2", (20., 0.), vec![(10, 20)], 0),
                create_delivery_job_with_times("job3", (30., 0.), vec![(5, 8), (100, 120)], 0),
                create_delivery_job_with_times("job4", (40., 0.), vec![(30, 40)], 0),
                create_delivery_job_with_times("job5", (50., 0.), vec![(40, 50)], 0),
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
                            .schedule_stamp(0, 0)
                            .load(vec![5])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(10, 10)
                            .load(vec![4])
                            .distance(10)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(20, 20)
                            .load(vec![3])
                            .distance(20)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((40., 0.))
                            .schedule_stamp(40, 40)
                            .load(vec![2])
                            .distance(40)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((50., 0.))
                            .schedule_stamp(50, 50)
                            .load(vec![1])
                            .distance(50)
                            .build_single("job5", "delivery"),
                        StopBuilder::default()
                            .coordinate((30., 0.))
                            .schedule_stamp(70, 100)
                            .load(vec![0])
                            .distance(70)
                            .build_single_time("job3", "delivery", (100, 100)),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(130, 130)
                            .load(vec![0])
                            .distance(100)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(100).waiting(30).build())
                    .build()
            )
            .build()
    );
}
