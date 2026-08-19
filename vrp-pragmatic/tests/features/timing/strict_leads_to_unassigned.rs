use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_have_unassigned_jobs_because_of_strict_times() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 10)], 0.),
                create_delivery_job_with_times("job2", (20., 0.), vec![(10, 20)], 0.),
                create_delivery_job_with_times("job3", (30., 0.), vec![(20, 30)], 0.),
                create_delivery_job_with_times("job4", (40., 0.), vec![(30, 40)], 0.),
                create_delivery_job_with_times("job5", (50., 0.), vec![(0, 10)], 0.),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let expected = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![4]).build_departure(),
                    StopBuilder::default()
                        .coordinate((10., 0.))
                        .schedule_stamp(10., 10.)
                        .load(vec![3])
                        .distance(10)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((20., 0.))
                        .schedule_stamp(20., 20.)
                        .load(vec![2])
                        .distance(20)
                        .build_single("job2", "delivery"),
                    StopBuilder::default()
                        .coordinate((30., 0.))
                        .schedule_stamp(30., 30.)
                        .load(vec![1])
                        .distance(30)
                        .build_single("job3", "delivery"),
                    StopBuilder::default()
                        .coordinate((40., 0.))
                        .schedule_stamp(40., 40.)
                        .load(vec![0])
                        .distance(40)
                        .build_single("job4", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(80., 80.)
                        .load(vec![0])
                        .distance(80)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(80).build())
                .build(),
        )
        .build();

    // The tour is asserted in full: every window here admits exactly one arrival, so there is no
    // tie for the solver to break differently.
    assert_eq!(solution.tours, expected.tours);

    // Only that the time window is among job5's reasons — see the note in `job_times`.
    let unassigned = solution.unassigned.expect("job5 must be reported as unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0].job_id, "job5");
    assert!(
        unassigned[0].reasons.iter().any(|reason| reason.code == "TIME_WINDOW_CONSTRAINT"),
        "the time window must be given as a reason: {:?}",
        unassigned[0].reasons
    );
}
