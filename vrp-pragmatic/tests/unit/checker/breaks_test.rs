use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn test_violations() -> Option<Vec<Violation>> {
    Some(vec![Violation::Break { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }])
}

fn get_matched_break_error_msg(matched: usize, actual: usize) -> Result<(), Vec<String>> {
    Err(vec![format!(
        "cannot match all breaks, matched: '{}', actual '{}' for vehicle 'my_vehicle_1', shift index '0'",
        matched, actual
    )])
}

fn get_total_break_error_msg(expected: usize, actual: usize) -> Result<(), Vec<String>> {
    Err(vec![format!(
        "amount of breaks does not match, expected: '{}', got '{}' for vehicle 'my_vehicle_1', shift index '0'",
        expected, actual
    )])
}

fn get_offset_break(start: f64, end: f64) -> VehicleBreakTime {
    VehicleBreakTime::TimeOffset(vec![start, end])
}

fn get_time_break(start: f64, end: f64) -> VehicleBreakTime {
    VehicleBreakTime::TimeWindow(vec![format_time(start), format_time(end)])
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
    break_times: VehicleBreakTime,
    violations: Option<Vec<Violation>>,
    has_break: bool,
    expected_result: Result<(), Vec<String>>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![2., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: vec![0., 0.].to_loc() },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(1000.).to_string(),
                        location: vec![0., 0.].to_loc(),
                    }),
                    dispatch: None,
                    breaks: Some(vec![VehicleBreak {
                        time: break_times,
                        places: vec![VehicleBreakPlace { duration: 2.0, location: None, tag: None }],
                        policy: None,
                    }]),
                    reloads: None,
                }],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
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

    let solution = Solution {
        statistic: Statistic {
            cost: 22.,
            distance: 4,
            duration: 8,
            times: Timing { driving: 4, serving: 2, break_time: 2, ..Timing::default() },
        },
        tours: vec![Tour {
            vehicle_id: "my_vehicle_1".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![
                create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    2,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ),
                create_stop_with_activity(
                    "job1",
                    "delivery",
                    (1., 0.),
                    1,
                    ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    5,
                ),
                Stop {
                    location: vec![2., 0.].to_loc(),
                    time: Schedule {
                        arrival: "1970-01-01T00:00:03Z".to_string(),
                        departure: "1970-01-01T00:00:06Z".to_string(),
                    },
                    distance: 2,
                    load: vec![0],
                    activities,
                },
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    0,
                    ("1970-01-01T00:00:08Z", "1970-01-01T00:00:08Z"),
                    4,
                ),
            ],
            statistic: Statistic {
                cost: 22.,
                distance: 4,
                duration: 8,
                times: Timing { driving: 4, serving: 2, break_time: 2, ..Timing::default() },
            },
        }],
        violations,
        ..create_empty_solution()
    };
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_breaks(&ctx);

    assert_eq!(result, expected_result);
}
