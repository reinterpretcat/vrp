use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn test_violations() -> Option<Vec<Violation>> {
    Some(vec![Violation::Break { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }])
}

fn get_matched_break_error_msg(matched: usize, actual: usize) -> Result<(), Vec<GenericError>> {
    Err(vec![format!(
        "cannot match all breaks, matched: '{matched}', actual '{actual}' for vehicle 'my_vehicle_1', shift index '0'"
    )
    .into()])
}

fn get_total_break_error_msg(expected: usize, actual: usize) -> Result<(), Vec<GenericError>> {
    Err(vec![format!(
        "amount of breaks does not match, expected: '{expected}', got '{actual}' for vehicle 'my_vehicle_1', shift index '0'"
    ).into()])
}

fn get_offset_break(start: Float, end: Float) -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeOffset(vec![start, end])
}

fn get_time_break(start: Float, end: Float) -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeWindow(vec![format_time(start), format_time(end)])
}

parameterized_test! {can_check_breaks, (break_times, violations, has_break, expected_result), {
    can_check_breaks_impl(break_times, violations, has_break, expected_result);
}}

can_check_breaks! {
    case01: (get_offset_break(2., 5.), None, true, Ok(())),
    case02: (get_offset_break(2., 5.), test_violations(), true, get_total_break_error_msg(1, 2)),
    case03: (get_offset_break(2., 5.), None, false, get_total_break_error_msg(1, 0)),

    case04: (get_offset_break(3., 6.), None, true, Ok(())),

    case05: (get_offset_break(0., 1.), None, true, get_matched_break_error_msg(0, 1)),
    case06: (get_offset_break(0., 1.), test_violations(), true, get_matched_break_error_msg(0, 1)),
    case07: (get_offset_break(0., 1.), None, false, get_total_break_error_msg(1, 0)),
    case08: (get_offset_break(0., 1.), test_violations(), false, Ok(())),

    case09: (get_offset_break(0., 1.), None, true, get_matched_break_error_msg(0, 1)),

    case10: (get_offset_break(7., 10.), test_violations(), false, Ok(())),
    case11: (get_offset_break(7., 10.), None, true, get_matched_break_error_msg(0, 1)),

    case12: (get_time_break(2., 5.), None, true, Ok(())),

    case13: (get_time_break(3., 6.), None, true, Ok(())),
    case14: (get_time_break(3., 6.), test_violations(), true, get_total_break_error_msg(1, 2)),

    case15: (get_time_break(0., 1.), test_violations(), false, Ok(())),
    case16: (get_time_break(0., 1.), test_violations(), true, get_matched_break_error_msg(0, 1)),
    case17: (get_time_break(0., 1.), None, true, get_matched_break_error_msg(0, 1)),

    case18: (get_time_break(7., 10.), test_violations(), false, Ok(())),
    case19: (get_time_break(7., 10.), test_violations(), true, get_matched_break_error_msg(0, 1)),
    case20: (get_time_break(7., 10.), None, true, get_matched_break_error_msg(0, 1)),
}

fn can_check_breaks_impl(
    break_times: VehicleOptionalBreakTime,
    violations: Option<Vec<Violation>>,
    has_break: bool,
    expected_result: Result<(), Vec<GenericError>>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (2., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: break_times,
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    reloads: None,
                    recharges: None,
                    job_times: None,
                }],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let mut activities = vec![Activity {
        job_id: "job2".to_string(),
        activity_type: "delivery".to_string(),
        location: None,
        time: Some(Interval { start: "1970-01-01T00:00:03Z".to_string(), end: "1970-01-01T00:00:04Z".to_string() }),
        job_tag: None,
        commute: None,
    }];
    if has_break {
        activities.push(Activity {
            job_id: "break".to_string(),
            activity_type: "break".to_string(),
            location: None,
            time: Some(Interval { start: "1970-01-01T00:00:04Z".to_string(), end: "1970-01-01T00:00:06Z".to_string() }),
            job_tag: None,
            commute: None,
        });
    }

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
                        .coordinate((2., 0.))
                        .schedule_stamp(3., 6.)
                        .load(vec![0])
                        .distance(2)
                        .activities(activities)
                        .build(),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(8., 8.)
                        .load(vec![0])
                        .distance(4)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(4).serving(2).break_time(2).build())
                .build(),
        )
        .violations(violations)
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_breaks(&ctx);

    assert_eq!(result, expected_result);
}

#[test]
fn can_match_a_break_between_two_activities_at_one_stop() {
    // A stop holding [job1, break, job2]: the break is neither the first nor the last
    // activity at that stop. The old windows(2) matcher counted it twice - once as `to`
    // in the window [job1, break], once as `from` in the window [break, job2] - even
    // though there is only one break activity to match.
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: get_time_break(2., 4.),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_open_vehicle_shift()
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
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(1., 5.)
                        .load(vec![0])
                        .distance(1)
                        .activity(ActivityBuilder::delivery().job_id("job1").time_stamp(1., 2.).build())
                        .activity(ActivityBuilder::break_type().time_stamp(2., 4.).build())
                        .activity(ActivityBuilder::delivery().job_id("job2").time_stamp(4., 5.).build())
                        .build(),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(6., 6.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .build(),
        )
        .build();

    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_breaks(&ctx), Ok(()));
}

#[test]
fn can_match_a_break_that_leads_a_multi_activity_stop() {
    // A stop holding [break, job]: the break is the FIRST activity, so it has no true
    // predecessor and the location check falls back to the stop's own location.
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (1., 0.))], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: get_time_break(2., 4.),
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    ..create_default_open_vehicle_shift()
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
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(2., 5.)
                        .load(vec![0])
                        .distance(1)
                        .activity(ActivityBuilder::break_type().time_stamp(2., 4.).build())
                        .activity(ActivityBuilder::delivery().job_id("job1").time_stamp(4., 5.).build())
                        .build(),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(6., 6.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .build(),
        )
        .build();

    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_breaks(&ctx), Ok(()));
}

#[test]
fn can_match_all_breaks_in_the_reported_incident_shape() {
    // Reproduces the FIELDROUTING-2V incident: a stop holding
    // [break, job1, break, break, job2] - a leading break, followed by a job, followed by
    // two consecutive breaks, followed by a job. That is 3 real break activities.
    //
    // Under the pre-fix windows(2) matcher this stop alone produced
    // "cannot match all breaks, matched: '4', actual '3'": the leading break and the
    // middle break were each counted once, but the third break (index 3) was counted
    // twice - once as `to` in the window [break, break], once as `from` in the window
    // [break, job2]. Passing here pins the count at 3, not 4.
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                    breaks: Some(vec![
                        VehicleBreak::Optional {
                            time: get_time_break(2., 4.),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy: None,
                        },
                        VehicleBreak::Optional {
                            time: get_time_break(6., 8.),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy: None,
                        },
                        VehicleBreak::Optional {
                            time: get_time_break(9., 11.),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy: None,
                        },
                    ]),
                    ..create_default_open_vehicle_shift()
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
                        .schedule_stamp(2., 12.)
                        .load(vec![0])
                        .distance(1)
                        .activity(ActivityBuilder::break_type().time_stamp(2., 4.).build())
                        .activity(ActivityBuilder::delivery().job_id("job1").time_stamp(4., 5.).build())
                        .activity(ActivityBuilder::break_type().time_stamp(6., 8.).build())
                        .activity(ActivityBuilder::break_type().time_stamp(9., 11.).build())
                        .activity(ActivityBuilder::delivery().job_id("job2").time_stamp(11., 12.).build())
                        .build(),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(14., 14.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .build(),
        )
        .build();

    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    assert_eq!(check_breaks(&ctx), Ok(()));
}
