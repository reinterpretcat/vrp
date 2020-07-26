use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_problem_with_depots(depots: Option<Vec<VehicleCargoPlace>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![3., 0.]), create_delivery_job("job2", vec![5., 0.])],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift { depots, ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    }
}

#[test]
fn can_assign_single_depot() {
    let problem = create_problem_with_depots(Some(vec![VehicleCargoPlace {
        location: vec![7., 0.].to_loc(),
        duration: 2.0,
        times: Some(vec![vec![format_time(10.), format_time(15.)]]),
        tag: None,
    }]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 42.,
                distance: 14,
                duration: 18,
                times: Timing { driving: 14, serving: 4, waiting: 0, break_time: 0 },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:03Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "depot",
                        "depot",
                        (7., 0.),
                        2,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:12Z"),
                        7,
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (5., 0.),
                        1,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                        9,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (3., 0.),
                        0,
                        ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                        11
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:21Z", "1970-01-01T00:00:21Z"),
                        14
                    )
                ],
                statistic: Statistic {
                    cost: 42.,
                    distance: 14,
                    duration: 18,
                    times: Timing { driving: 14, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_handle_unassignable_depot() {
    let problem = create_problem_with_depots(Some(vec![VehicleCargoPlace {
        location: vec![1001., 0.].to_loc(),
        duration: 2.0,
        times: Some(vec![vec![format_time(10.), format_time(15.)]]),
        tag: None,
    }]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.is_empty());
    assert_eq!(solution.unassigned.map_or(0, |u| u.len()), 2);
}

parameterized_test! {can_handle_two_depots, location, {
    can_handle_two_depots_impl(location);
}}

can_handle_two_depots! {
    case01: &[7., 0.],
    case02: &[1001., 0.],
}

fn can_handle_two_depots_impl(location: &[f64]) {
    let problem = create_problem_with_depots(Some(vec![
        VehicleCargoPlace {
            location: location.to_vec().to_loc(),
            duration: 6.,
            times: Some(vec![vec![format_time(0.), format_time(1000.)]]),
            tag: None,
        },
        VehicleCargoPlace { location: vec![8., 0.].to_loc(), duration: 1., times: None, tag: None },
    ]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours[0].stops[1].location, vec![8., 0.].to_loc());
    assert_eq!(solution.tours[0].stops[1].activities[0].activity_type, "depot");
    assert_eq!(
        solution.statistic,
        Statistic {
            cost: 45.,
            distance: 16,
            duration: 19,
            times: Timing { driving: 16, serving: 3, waiting: 0, break_time: 0 },
        }
    );
}
