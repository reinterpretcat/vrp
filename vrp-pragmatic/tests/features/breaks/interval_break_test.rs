use crate::format::problem::Objective::{MinimizeCost, MinimizeUnassigned};
use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

fn create_test_objectives() -> Option<Vec<Objective>> {
    Some(vec![MinimizeUnassigned { breaks: Some(10.) }, MinimizeCost])
}

#[test]
fn can_assign_interval_break_between_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (15., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(0.)),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeOffset(vec![5., 10.]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_test_objectives(),
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
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 8.)
                            .load(vec![1])
                            .distance(5)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job1")
                                    .coordinate((5., 0.))
                                    .time_stamp(5., 6.)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().coordinate((5., 0.)).time_stamp(6., 8.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((15., 0.))
                            .schedule_stamp(18., 19.)
                            .load(vec![0])
                            .distance(15)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(34., 34.)
                            .load(vec![0])
                            .distance(30)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(30).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_assign_interval_break_with_reload() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (10., 0.)),
                create_delivery_job("job2", (15., 0.)),
                create_delivery_job("job3", (20., 0.)),
                create_delivery_job("job4", (25., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: Some(format_time(0.)),
                        location: (0., 0.).to_loc(),
                    },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (30., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeOffset(vec![8., 12.]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    reloads: Some(vec![VehicleReload {
                        times: Some(vec![vec![format_time(0.), format_time(1000.)]]),
                        location: (0., 0.).to_loc(),
                        duration: 3.0,
                        ..create_default_reload()
                    }]),
                    recharges: None,
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_test_objectives(),
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
                            .coordinate((10., 0.))
                            .schedule_stamp(10., 13.)
                            .load(vec![1])
                            .distance(10)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job1")
                                    .coordinate((10., 0.))
                                    .time_stamp(10., 11.)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().coordinate((10., 0.)).time_stamp(11., 13.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((15., 0.))
                            .schedule_stamp(18., 19.)
                            .load(vec![0])
                            .distance(15)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(34., 37.)
                            .load(vec![2])
                            .distance(30)
                            .build_single("reload", "reload"),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(57., 58.)
                            .load(vec![1])
                            .distance(50)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((25., 0.))
                            .schedule_stamp(63., 64.)
                            .load(vec![0])
                            .distance(55)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((30., 0.))
                            .schedule_stamp(69., 69.)
                            .load(vec![0])
                            .distance(60)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(60).serving(7).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
#[ignore]
fn can_consider_departure_rescheduling() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (5., 0.), vec![(10, 10)], 1.),
                create_delivery_job_with_times("job2", (10., 0.), vec![(10, 30)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeOffset(vec![10., 12.]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_test_objectives(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 2000);

    assert!(solution.violations.is_none());
    assert!(solution.unassigned.is_none());
}
