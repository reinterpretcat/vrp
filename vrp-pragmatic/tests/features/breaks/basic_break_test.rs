use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_break_between_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 2.0,
                            location: Some((6., 0.).to_loc()),
                            tag: Some("break_tag".to_string()),
                        }],
                        policy: None,
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
        Solution {
            statistic: Statistic {
                cost: 54.,
                distance: 20,
                duration: 24,
                times: Timing { driving: 20, serving: 2, break_time: 2, ..Timing::default() },
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
                        (5., 0.),
                        1,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        5,
                    ),
                    create_stop_with_activity_with_tag(
                        "break",
                        "break",
                        (6., 0.),
                        1,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:09Z"),
                        6,
                        "break_tag"
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:24Z", "1970-01-01T00:00:24Z"),
                        20
                    )
                ],
                statistic: Statistic {
                    cost: 54.,
                    distance: 20,
                    duration: 24,
                    times: Timing { driving: 20, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
