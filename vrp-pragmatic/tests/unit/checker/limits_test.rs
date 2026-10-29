use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn create_test_problem(limits: Option<VehicleLimits>) -> Problem {
    create_test_problem_with_span(limits, None)
}

fn create_test_problem_with_span(limits: Option<VehicleLimits>, span: Option<RouteCostSpan>) -> Problem {
    let vehicle = create_default_vehicle_type();

    Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["some_real_vehicle".to_string()],
                costs: VehicleCosts { span, ..vehicle.costs.clone() },
                limits,
                ..vehicle
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

fn create_test_solution(statistic: Statistic, stops: Vec<Stop>) -> Solution {
    SolutionBuilder::default()
        .tour(Tour {
            vehicle_id: "some_real_vehicle".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops,
            statistic,
        })
        .build()
}

parameterized_test! {can_check_shift_and_distance_limit, (max_distance, shift_time, actual, expected_result), {
    let expected_result = if let Err(prefix_msg) = expected_result {
        Err(format!(
            "{} violation, expected: not more than {}, got: {}, vehicle id 'some_real_vehicle', shift index: 0",
            prefix_msg, max_distance.unwrap_or_else(|| shift_time.unwrap()), actual,
        ).into())
    } else {
        Ok(())
    };
    can_check_shift_and_distance_limit_impl(max_distance, shift_time, actual, expected_result);
}}

can_check_shift_and_distance_limit! {
    case_01: (Some(10.), None, 11, Result::<(), _>::Err("max distance limit")),
    case_02: (Some(10.), None, 10, Result::<_, &str>::Ok(())),
    case_03: (Some(10.), None, 9, Result::<_, &str>::Ok(())),

    case_04: (None, Some(10.), 11, Result::<(), _>::Err("shift time limit")),
    case_05: (None, Some(10.), 10, Result::<_, &str>::Ok(())),
    case_06: (None, Some(10.), 9, Result::<_, &str>::Ok(())),

    case_07: (None, None, i64::MAX, Result::<_, &str>::Ok(())),
}

pub fn can_check_shift_and_distance_limit_impl(
    max_distance: Option<Float>,
    max_duration: Option<Float>,
    actual: i64,
    expected: Result<(), GenericError>,
) {
    let problem =
        create_test_problem(Some(VehicleLimits { max_distance, max_duration, tour_size: None, min_tour_size: None }));
    // The distance limit is read off the statistic, the duration limit off the tour itself, so only
    // the duration cases need stops - and only they can have them: `case_07` states no limit at all
    // with an `i64::MAX` statistic, which is not a timestamp any schedule can carry.
    let stops = if max_duration.is_some() {
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![0]).build_departure(),
            StopBuilder::default()
                .coordinate((0., 0.))
                .schedule_stamp(actual as Float, actual as Float)
                .load(vec![0])
                .distance(actual)
                .build_arrival(),
        ]
    } else {
        vec![]
    };
    let solution =
        create_test_solution(Statistic { distance: actual, duration: actual, ..Statistic::default() }, stops);
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_shift_limits(&ctx);

    assert_eq!(result, expected);
}

/// A tour with an hour of commute at each end: depot 00:00, first job 01:00-02:00, last job
/// 03:00-04:00, depot 05:00. Round trip five hours, jobs only three.
fn create_spanned_tour_stops() -> Vec<Stop> {
    vec![
        StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
        StopBuilder::default()
            .coordinate((1., 0.))
            .schedule_stamp(3600., 7200.)
            .load(vec![1])
            .distance(1)
            .build_single("job1", "delivery"),
        StopBuilder::default()
            .coordinate((2., 0.))
            .schedule_stamp(10800., 14400.)
            .load(vec![0])
            .distance(2)
            .build_single("job2", "delivery"),
        StopBuilder::default()
            .coordinate((0., 0.))
            .schedule_stamp(18000., 18000.)
            .load(vec![0])
            .distance(4)
            .build_arrival(),
    ]
}

parameterized_test! {can_measure_the_duration_limit_over_the_paid_span, (span, expected_duration), {
    can_measure_the_duration_limit_over_the_paid_span_impl(span, expected_duration);
}}

can_measure_the_duration_limit_over_the_paid_span! {
    case_01_default: (None, 18000.),
    case_02_depot_to_depot: (Some(RouteCostSpan::DepotToDepot), 18000.),
    case_03_depot_to_last_job: (Some(RouteCostSpan::DepotToLastJob), 14400.),
    case_04_first_job_to_depot: (Some(RouteCostSpan::FirstJobToDepot), 14400.),
    case_05_first_job_to_last_job: (Some(RouteCostSpan::FirstJobToLastJob), 10800.),
}

/// `maxDuration` caps the time the shift is PAID for, so it has to be read over the same stretch
/// the solver enforces it on - the vehicle's `costs.span`. A cap of zero makes each span report
/// its own measurement back in the error, which is what pins them here.
pub fn can_measure_the_duration_limit_over_the_paid_span_impl(span: Option<RouteCostSpan>, expected_duration: Float) {
    let problem = create_test_problem_with_span(
        Some(VehicleLimits { max_distance: None, max_duration: Some(0.), tour_size: None, min_tour_size: None }),
        span,
    );
    let solution =
        create_test_solution(Statistic { duration: 18000, ..Statistic::default() }, create_spanned_tour_stops());
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(
        check_shift_limits(&ctx),
        Err(format!(
            "shift time limit violation, expected: not more than 0, got: {expected_duration}, \
             vehicle id 'some_real_vehicle', shift index: 0"
        )
        .into())
    );
}

/// The shape production reports: the paid span fits the cap and the two commute legs push the
/// round trip past it. `statistic.duration` is always the round trip, so reading the cap against
/// it fails a tour the solver built exactly to the limit it was given.
#[test]
pub fn can_pass_a_tour_whose_commute_pushes_the_round_trip_over_the_cap() {
    let problem = create_test_problem_with_span(
        Some(VehicleLimits { max_distance: None, max_duration: Some(14400.), tour_size: None, min_tour_size: None }),
        Some(RouteCostSpan::FirstJobToLastJob),
    );
    let solution =
        create_test_solution(Statistic { duration: 18000, ..Statistic::default() }, create_spanned_tour_stops());
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_shift_limits(&ctx), Ok(()));
}

/// A break is not a visit, so it can neither open nor close the paid span - the same reading the
/// tour size check beside this takes.
#[test]
pub fn can_leave_a_break_out_of_the_paid_span() {
    let problem = create_test_problem_with_span(
        Some(VehicleLimits { max_distance: None, max_duration: Some(0.), tour_size: None, min_tour_size: None }),
        Some(RouteCostSpan::FirstJobToLastJob),
    );
    let mut stops = create_spanned_tour_stops();
    stops.insert(
        1,
        StopBuilder::default()
            .coordinate((0., 0.))
            .schedule_stamp(1800., 3600.)
            .load(vec![2])
            .distance(0)
            .build_single("break", "break"),
    );
    stops.insert(
        stops.len() - 1,
        StopBuilder::default()
            .coordinate((2., 0.))
            .schedule_stamp(14400., 16200.)
            .load(vec![0])
            .distance(2)
            .build_single("break", "break"),
    );
    let solution = create_test_solution(Statistic { duration: 18000, ..Statistic::default() }, stops);
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(
        check_shift_limits(&ctx),
        Err("shift time limit violation, expected: not more than 0, got: 10800, vehicle id 'some_real_vehicle', shift index: 0"
            .into())
    );
}

#[test]
pub fn can_check_tour_size_limit() {
    let problem = create_test_problem(Some(VehicleLimits {
        max_distance: None,
        max_duration: None,
        tour_size: Some(2),
        min_tour_size: None,
    }));
    let solution = create_test_solution(
        Statistic::default(),
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![3]).build_departure(),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1., 1.)
                .load(vec![2])
                .distance(1)
                .build_single("job1", "delivery"),
            StopBuilder::default()
                .coordinate((2., 0.))
                .schedule_stamp(2., 2.)
                .load(vec![1])
                .distance(2)
                .build_single("job2", "delivery"),
            StopBuilder::default()
                .coordinate((3., 0.))
                .schedule_stamp(3., 3.)
                .load(vec![0])
                .distance(3)
                .build_single("job3", "delivery"),
            StopBuilder::default()
                .coordinate((0., 0.))
                .schedule_stamp(6., 6.)
                .load(vec![0])
                .distance(6)
                .build_arrival(),
        ],
    );
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_shift_limits(&ctx);

    assert_eq!(
        result,
        Err("tour size limit violation, expected: not more than 2, got: 3, vehicle id 'some_real_vehicle', shift index: 0"
            .into())
    );
}

/// A break on the tour is an activity, but not a stop: two stops and a break fit a
/// tour size of two, as they do for the solver's own constraint.
#[test]
pub fn can_leave_a_break_out_of_the_tour_size() {
    let problem = create_test_problem(Some(VehicleLimits {
        max_distance: None,
        max_duration: None,
        tour_size: Some(2),
        min_tour_size: None,
    }));
    let solution = create_test_solution(
        Statistic::default(),
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1., 1.)
                .load(vec![1])
                .distance(1)
                .build_single("job1", "delivery"),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1., 3.)
                .load(vec![1])
                .distance(1)
                .build_single("break", "break"),
            StopBuilder::default()
                .coordinate((2., 0.))
                .schedule_stamp(4., 4.)
                .load(vec![0])
                .distance(2)
                .build_single("job2", "delivery"),
            StopBuilder::default()
                .coordinate((0., 0.))
                .schedule_stamp(6., 6.)
                .load(vec![0])
                .distance(4)
                .build_arrival(),
        ],
    );
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_shift_limits(&ctx), Ok(()));
}

/// `minTourSize` feeds an objective, not a constraint, so an under-sized tour is a
/// worse solution and never an infeasible one — the checker must let it pass.
#[test]
pub fn can_ignore_min_tour_size_because_it_is_an_objective() {
    let problem = create_test_problem(Some(VehicleLimits {
        max_distance: None,
        max_duration: None,
        tour_size: None,
        min_tour_size: Some(3),
    }));
    let solution = create_test_solution(
        Statistic::default(),
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1., 1.)
                .load(vec![1])
                .distance(1)
                .build_single("job1", "delivery"),
            StopBuilder::default()
                .coordinate((2., 0.))
                .schedule_stamp(2., 2.)
                .load(vec![0])
                .distance(2)
                .build_single("job2", "delivery"),
            StopBuilder::default()
                .coordinate((0., 0.))
                .schedule_stamp(4., 4.)
                .load(vec![0])
                .distance(4)
                .build_arrival(),
        ],
    );
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_shift_limits(&ctx);

    assert_eq!(result, Ok(()));
}

#[test]
pub fn can_pass_min_tour_size_limit_when_satisfied() {
    let problem = create_test_problem(Some(VehicleLimits {
        max_distance: None,
        max_duration: None,
        tour_size: None,
        min_tour_size: Some(2),
    }));
    let solution = create_test_solution(
        Statistic::default(),
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1., 1.)
                .load(vec![1])
                .distance(1)
                .build_single("job1", "delivery"),
            StopBuilder::default()
                .coordinate((2., 0.))
                .schedule_stamp(2., 2.)
                .load(vec![0])
                .distance(2)
                .build_single("job2", "delivery"),
            StopBuilder::default()
                .coordinate((0., 0.))
                .schedule_stamp(4., 4.)
                .load(vec![0])
                .distance(4)
                .build_arrival(),
        ],
    );
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_shift_limits(&ctx);

    assert_eq!(result, Ok(()));
}

#[test]
fn can_check_shift_time() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(5, 10)], 1.)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(5.), location: (0., 0.).to_loc() }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(2., 2.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(5., 6.)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(7., 7.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(1).waiting(2).build())
                .build(),
        )
        .build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());
    let ctx = CheckerContext::new(core_problem, problem, None, solution).unwrap();

    let result = check_shift_time(&ctx);

    assert_eq!(result, Err("tour time is outside shift time, vehicle id 'my_vehicle_1', shift index: 0".into()));
}

#[test]
fn can_check_recharge_distance() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: None,
                    recharges: Some(VehicleRecharges {
                        max_distance: 8.,
                        stations: vec![VehicleRechargeStation {
                            location: (8., 0.).to_loc(),
                            duration: 0.,
                            times: None,
                            tag: None,
                        }],
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(1., 2.)
                        .load(vec![1])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((10., 0.))
                        .schedule_stamp(11., 12.)
                        .load(vec![0])
                        .distance(10)
                        .build_single("job2", "delivery"),
                ])
                .statistic(StatisticBuilder::default().driving(10).serving(2).waiting(0).build())
                .build(),
        )
        .build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());
    let ctx = CheckerContext::new(core_problem, problem, None, solution).unwrap();

    let result = check_recharge_limits(&ctx);

    assert_eq!(
        result,
        Err("recharge distance violation: expected limit is 8, got 10, vehicle id 'my_vehicle_1', shift index: 0"
            .into())
    );
}
