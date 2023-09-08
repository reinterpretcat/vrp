use crate::format::problem::*;
use crate::format::Location;
use crate::helpers::*;

#[test]
fn can_wait_for_job_start() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (1., 0.), vec![(0, 1)], 0.),
                create_delivery_job_with_times("job2", (2., 0.), vec![(10, 20)], 0.),
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
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 1.)
                            .load(vec![1])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2., 10.)
                            .load(vec![0])
                            .distance(2)
                            .build_single_time("job2", "delivery", (10., 10.)),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(12., 12.)
                            .load(vec![0])
                            .distance(4)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(4).waiting(8).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_skip_initial_waiting() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(10, 20)], 10.)],
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
                            .schedule_stamp(0., 9.)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(10., 20.)
                            .load(vec![0])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(21., 21.)
                            .load(vec![0])
                            .distance(2)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(2).serving(10).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_consider_latest_departure_time() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(10, 20)], 10.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T00:00:00Z".to_string(),
                        latest: Some("1970-01-01T00:00:05Z".to_string()),
                        location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                    },
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle("my_vehicle")
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
                            .schedule_stamp(0., 5.)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(6., 20.)
                            .load(vec![0])
                            .distance(1)
                            .build_single_time("job1", "delivery", (10., 20.)),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(21., 21.)
                            .load(vec![0])
                            .distance(2)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(2).serving(10).waiting(4).build())
                    .build()
            )
            .build()
    );
}
