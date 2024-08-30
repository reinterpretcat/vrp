use crate::format::problem::*;
use crate::format::solution::Violation;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::common::Timestamp;

parameterized_test! {can_skip_break_when_vehicle_not_used, policy, {
    can_skip_break_when_vehicle_not_used_impl(policy);
}}

can_skip_break_when_vehicle_not_used! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
    case_03: Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd),
}

fn can_skip_break_when_vehicle_not_used_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    shifts: vec![VehicleShift {
                        start: ShiftStart { earliest: format_time(0.), latest: None, location: (100., 0.).to_loc() },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(1000.),
                            location: (100., 0.).to_loc(),
                        }),
                        breaks: Some(vec![VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                            places: vec![VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((6., 0.).to_loc()),
                                tag: None,
                            }],
                            policy,
                        }]),
                        ..create_default_vehicle_shift()
                    }],
                    ..create_default_vehicle_type()
                },
                create_default_vehicle("vehicle_without_break"),
            ],
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
                    .type_id("vehicle_without_break")
                    .vehicle_id("vehicle_without_break_1")
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![1])
                            .distance(10)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(16., 17.)
                            .load(vec![0])
                            .distance(15)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(22., 22.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(2).build())
                    .build()
            )
            .build()
    );
}

parameterized_test! {can_skip_break_when_jobs_completed, policy, {
    can_skip_break_when_jobs_completed_impl(policy);
}}

can_skip_break_when_jobs_completed! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
    case_03: Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd),
}

fn can_skip_break_when_jobs_completed_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_duration("job1", (1., 0.), 10.)], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(8.)]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 2.0,
                            location: Some((6., 0.).to_loc()),
                            tag: None,
                        }],
                        policy,
                    }]),
                    ..create_default_vehicle_shift()
                }],
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
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 11.)
                            .load(vec![0])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(12., 12.)
                            .load(vec![0])
                            .distance(2)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(2).serving(10).build())
                    .build()
            )
            .violations(Some(vec![Violation::Break { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]))
            .build()
    );
}

parameterized_test! {can_skip_second_break_when_jobs_completed, policy, {
    can_skip_second_break_when_jobs_completed_impl(policy);
}}

can_skip_second_break_when_jobs_completed! {
    case_01: None,
    case_02: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
}

fn can_skip_second_break_when_jobs_completed_impl(policy: Option<VehicleOptionalBreakPolicy>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                            places: vec![VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((6., 0.).to_loc()),
                                tag: None,
                            }],
                            policy: policy.clone(),
                        },
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(100.), format_time(120.)]),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy,
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
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
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![1])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(7., 9.)
                            .load(vec![1])
                            .distance(6)
                            .build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(24., 24.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

parameterized_test! {can_skip_break_depending_on_policy, (policy, location, time, expected), {
    can_skip_break_depending_on_policy_impl(policy, location, time, expected);
}}

can_skip_break_depending_on_policy! {
    case_01: (Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd), 5., (5., 11.), 0),
    case_02: (Some(VehicleOptionalBreakPolicy::SkipIfArrivalBeforeEnd), 5., (5., 8.), 2),

    case_03: (Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection), 5., (5., 11.), 2),
    case_04: (Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection), 5., (5., 8.), 2),
}

fn can_skip_break_depending_on_policy_impl(
    policy: Option<VehicleOptionalBreakPolicy>,
    location: f64,
    time: (Timestamp, Timestamp),
    expected: i64,
) {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_duration("job1", (location, 0.), 0.)], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(time.0), format_time(time.1)]),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert!(solution.violations.is_none());
    assert_eq!(solution.statistic.times.break_time, expected);
}
