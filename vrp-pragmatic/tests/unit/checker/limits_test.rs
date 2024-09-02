use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn create_test_problem(limits: Option<VehicleLimits>) -> Problem {
    Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["some_real_vehicle".to_string()],
                limits,
                ..create_default_vehicle_type()
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
    case_01: (Some(10), None, 11, Result::<(), _>::Err("max distance limit")),
    case_02: (Some(10), None, 10, Result::<_, &str>::Ok(())),
    case_03: (Some(10), None, 9, Result::<_, &str>::Ok(())),

    case_04: (None, Some(10), 11, Result::<(), _>::Err("shift time limit")),
    case_05: (None, Some(10), 10, Result::<_, &str>::Ok(())),
    case_06: (None, Some(10), 9, Result::<_, &str>::Ok(())),

    case_07: (None, None, i64::MAX, Result::<_, &str>::Ok(())),
}

pub fn can_check_shift_and_distance_limit_impl(
    max_distance: Option<Distance>,
    max_duration: Option<Duration>,
    actual: i64,
    expected: Result<(), GenericError>,
) {
    let problem = create_test_problem(Some(VehicleLimits { max_distance, max_duration, tour_size: None }));
    let solution =
        create_test_solution(Statistic { distance: actual, duration: actual, ..Statistic::default() }, vec![]);
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_shift_limits(&ctx);

    assert_eq!(result, expected);
}

#[test]
pub fn can_check_tour_size_limit() {
    let problem =
        create_test_problem(Some(VehicleLimits { max_distance: None, max_duration: None, tour_size: Some(2) }));
    let solution = create_test_solution(
        Statistic::default(),
        vec![
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0, 0).load(vec![3]).build_departure(),
            StopBuilder::default()
                .coordinate((1., 0.))
                .schedule_stamp(1, 1)
                .load(vec![2])
                .distance(1)
                .build_single("job1", "delivery"),
            StopBuilder::default()
                .coordinate((2., 0.))
                .schedule_stamp(2, 2)
                .load(vec![1])
                .distance(2)
                .build_single("job2", "delivery"),
            StopBuilder::default()
                .coordinate((3., 0.))
                .schedule_stamp(3, 3)
                .load(vec![0])
                .distance(3)
                .build_single("job3", "delivery"),
            StopBuilder::default().coordinate((0., 0.)).schedule_stamp(6, 6).load(vec![0]).distance(6).build_arrival(),
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

#[test]
fn can_check_shift_time() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(5, 10)], 1)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(5), location: (0., 0.).to_loc() }),
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
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(2, 2).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(5, 6)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(7, 7)
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
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: None,
                    recharges: Some(VehicleRecharges {
                        max_distance: 8,
                        stations: vec![VehicleRechargeStation {
                            location: (8., 0.).to_loc(),
                            duration: 0,
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
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0, 0).load(vec![2]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(1, 2)
                        .load(vec![1])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((10., 0.))
                        .schedule_stamp(11, 12)
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
