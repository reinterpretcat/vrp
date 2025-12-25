use crate::format::Location;
use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

fn create_shift_start() -> ShiftStart {
    ShiftStart { earliest: format_time(0.), latest: Some(format_time(0.)), location: (0., 0.).to_loc() }
}

fn create_problem(jobs: Vec<Job>, vehicle_break: VehicleBreak, is_open: bool) -> Problem {
    let vehicle_shift = if is_open { create_default_open_vehicle_shift() } else { create_default_vehicle_shift() };
    Problem {
        plan: Plan { jobs, ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: create_shift_start(),
                    breaks: Some(vec![vehicle_break]),
                    ..vehicle_shift
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

#[test]
fn can_assign_break_during_travel() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
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
                        StopBuilder::new_transit().schedule_stamp(7., 9.).load(vec![1]).build_single("break", "break"),
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

#[test]
fn can_assign_break_during_activity() {
    let is_open = false;
    let problem = create_problem(
        vec![create_delivery_job_with_duration("job1", (5., 0.), 3.)],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(7.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
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
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 10.)
                            .load(vec![0])
                            .distance(5)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job1")
                                    .coordinate((5., 0.))
                                    .time_stamp(5., 10.)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().time_stamp(7., 9.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(15., 15.)
                            .load(vec![0])
                            .distance(10)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(3).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_handle_required_break_when_its_start_falls_at_activity_end() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(6.), latest: format_time(6.) },
            duration: 2.,
        },
        is_open,
    );
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
                        StopBuilder::new_transit().schedule_stamp(6., 8.).load(vec![1]).build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_skip_break_if_it_is_after_start_before_end_range() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(5.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(get_ids_from_tour(&solution.tours[0]).iter().flatten().all(|id| id != "break"));
}

#[test]
fn can_reschedule_break_early_from_transport_to_activity() {
    let is_open = true;
    let problem = create_problem(
        vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
        VehicleBreak::Required {
            time: VehicleRequiredBreakTime::ExactTime { earliest: format_time(4.), latest: format_time(7.) },
            duration: 2.,
        },
        is_open,
    );
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
                            .activity(ActivityBuilder::break_type().time_stamp(6., 8.).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_handle_required_break_with_infeasible_sequence_relation() {
    let create_test_job = |index: usize, duration: f64, times: (String, String)| Job {
        services: Some(vec![JobTask {
            places: vec![JobPlace {
                location: Location::Reference { index },
                duration,
                times: Some(vec![vec![times.0, times.1]]),
                tag: None,
            }],
            demand: None,
            order: None,
        }]),
        ..create_job(index.to_string().as_str())
    };

    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_test_job(0, 10800., (format_time(0.), format_time(86399.))),
                create_test_job(1, 3600., (format_time(81000.), format_time(81000.))),
                create_test_job(2, 1800., (format_time(86400. + 900.), format_time(86400. + 900.))),
                create_test_job(3, 5400., (format_time(75600.), format_time(75600.))),
                create_test_job(4, 1800., (format_time(86400. + 2700.), format_time(86400. + 2700.))),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Sequence,
                jobs: to_strings(vec!["3", "1", "2", "4"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: Some(0),
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(86400. + 28800.),
                        latest: Some(format_time(86400. + 28800.)),
                        location: Location::Reference { index: 5 },
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(86400. + 57600.),
                        location: Location::Reference { index: 5 },
                    }),
                    breaks: Some(vec![VehicleBreak::Required {
                        time: VehicleRequiredBreakTime::OffsetTime { earliest: 15303., latest: 15303. },
                        duration: 1800.,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let matrix = Matrix {
        profile: Some("car".to_string()),
        timestamp: None,
        travel_times: vec![
            0, 635, 24, 580, 27, 2232, 625, 0, 650, 76, 653, 2507, 24, 660, 0, 605, 3, 2257, 570, 95, 595, 0, 598,
            2449, 27, 663, 3, 608, 0, 2260, 2232, 2545, 2257, 2515, 2260, 0,
        ],
        distances: vec![
            0, 8888, 192, 8510, 215, 52931, 8896, 0, 9088, 450, 9111, 56579, 192, 9080, 0, 8702, 23, 53123, 8518, 450,
            8710, 0, 8733, 60163, 215, 9103, 23, 8725, 0, 53146, 52996, 56684, 53188, 60477, 53211, 0,
        ],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    // Basic assertion - no crash, solution should exist and have at least one tour
    assert!(!solution.tours.is_empty());
}
