use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_break_using_second_location() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![10., 0.]), create_delivery_job("job2", vec![20., 0.])],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(1000.).to_string(),
                        location: vec![30., 0.].to_loc(),
                    }),
                    breaks: Some(vec![VehicleBreak {
                        time: VehicleBreakTime::TimeWindow(vec![format_time(10.), format_time(30.)]),
                        duration: 2.0,
                        locations: Some(vec![vec![1., 0.].to_loc(), vec![11., 0.].to_loc()]),
                        tag: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
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
                times: Timing { driving: 30, serving: 2, waiting: 0, break_time: 2 },
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
                    create_stop_with_activity(
                        "break",
                        "break",
                        (11., 0.),
                        1,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:14Z"),
                        11,
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
                    times: Timing { driving: 30, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
