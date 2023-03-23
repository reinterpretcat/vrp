use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_break_using_second_place() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (10., 0.)), create_delivery_job("job2", (20., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (30., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(10.), format_time(30.)]),
                        places: vec![
                            VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((1., 0.).to_loc()),
                                tag: Some("first".to_string()),
                            },
                            VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((11., 0.).to_loc()),
                                tag: Some("second".to_string()),
                            },
                        ],
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
                cost: 74.,
                distance: 30,
                duration: 34,
                times: Timing { driving: 30, serving: 2, break_time: 2, ..Timing::default() },
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
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        10,
                    ),
                    create_stop_with_activity_with_tag(
                        "break",
                        "break",
                        (11., 0.),
                        1,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:14Z"),
                        11,
                        "second",
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (20., 0.),
                        0,
                        ("1970-01-01T00:00:23Z", "1970-01-01T00:00:24Z"),
                        20,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (30., 0.),
                        0,
                        ("1970-01-01T00:00:34Z", "1970-01-01T00:00:34Z"),
                        30,
                    )
                ],
                statistic: Statistic {
                    cost: 74.,
                    distance: 30,
                    duration: 34,
                    times: Timing { driving: 30, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
