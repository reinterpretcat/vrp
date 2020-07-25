use super::*;
use crate::format_time;
use crate::helpers::*;

fn test_violations() -> Option<Vec<Violation>> {
    Some(vec![Violation::Break {
        vehicle_id: "my_vehicle_1".to_string(),
        shift_index: 0,
        reason: "some reason".to_string(),
    }])
}

parameterized_test! {can_check_breaks, (break_times, violations, expected_result), {
    can_check_breaks_impl(break_times, violations, expected_result);
}}

can_check_breaks! {
    case01: (VehicleBreakTime::TimeOffset(vec![2., 5.]), None, Ok(())),
    case02: (VehicleBreakTime::TimeOffset(vec![3., 6.]), None, Ok(())),

    case03: (VehicleBreakTime::TimeOffset(vec![0., 1.]), test_violations(), Ok(())),
    case04: (VehicleBreakTime::TimeOffset(vec![0., 1.]), None,
            Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),

    case05: (VehicleBreakTime::TimeOffset(vec![7., 10.]), test_violations(), Ok(())),
    case06: (VehicleBreakTime::TimeOffset(vec![7., 10.]), None,
            Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),

    case07: (VehicleBreakTime::TimeWindow(vec![format_time(2.), format_time(5.)]), None, Ok(())),
    case08: (VehicleBreakTime::TimeWindow(vec![format_time(3.), format_time(6.)]), None, Ok(())),

    case09: (VehicleBreakTime::TimeWindow(vec![format_time(0.), format_time(1.)]), test_violations(), Ok(())),
    case10: (VehicleBreakTime::TimeWindow(vec![format_time(0.), format_time(1.)]), None,
             Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),

    case11: (VehicleBreakTime::TimeWindow(vec![format_time(7.), format_time(10.)]), test_violations(), Ok(())),
    case12: (VehicleBreakTime::TimeWindow(vec![format_time(7.), format_time(10.)]), None,
             Err("Amount of breaks does not match, expected: '1', got '0'".to_owned())),
}

fn can_check_breaks_impl(
    break_times: VehicleBreakTime,
    violations: Option<Vec<Violation>>,
    expected_result: Result<(), String>,
) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![2., 0.])],
            relations: None,
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
                    depots: None,
                    breaks: Some(vec![VehicleBreak { time: break_times, duration: 0.0, locations: None }]),
                    reloads: None,
                }],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let solution = Solution {
        statistic: Statistic {
            cost: 22.,
            distance: 4,
            duration: 8,
            times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 2 },
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
                    activities: vec![
                        Activity {
                            job_id: "job2".to_string(),
                            activity_type: "delivery".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                        },
                        Activity {
                            job_id: "break".to_string(),
                            activity_type: "break".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                        },
                    ],
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
                times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 2 },
            },
        }],
        violations,
        ..create_empty_solution()
    };

    let result = check_breaks(&CheckerContext::new(problem, None, solution));

    assert_eq!(result, expected_result);
}
