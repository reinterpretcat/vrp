use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

parameterized_test! {can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload, generations, {
    can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload_impl(generations);
}}

can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload! {
    case01: 1,
    case02: 200,
}

fn can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload_impl(generations: usize) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple", vec![1., 0.]),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((6., 0.), 1., vec![2])],
                ),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: vec![0., 0.].to_loc() },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(100.).to_string(),
                        location: vec![0., 0.].to_loc(),
                    }),
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: vec![0., 0.].to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), generations);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 46.,
                distance: 16,
                duration: 20,
                times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
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
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "simple",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2,
                        "p1"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (8., 0.),
                        2,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        8,
                        "p2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                        10,
                        "d1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:20Z"),
                        16
                    )
                ],
                statistic: Statistic {
                    cost: 46.,
                    distance: 16,
                    duration: 20,
                    times: Timing { driving: 16, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
