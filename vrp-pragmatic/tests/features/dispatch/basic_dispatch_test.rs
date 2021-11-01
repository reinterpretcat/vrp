use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_problem_with_dispatch(dispatch: Option<Vec<VehicleDispatch>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![3., 0.]), create_delivery_job("job2", vec![5., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift { dispatch: dispatch, ..create_default_vehicle_shift() }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    }
}

#[test]
fn can_assign_single_dispatch() {
    let problem = create_problem_with_dispatch(Some(vec![VehicleDispatch {
        location: vec![7., 0.].to_loc(),
        limits: vec![VehicleDispatchLimit { max: 1, start: format_time(10.), end: format_time(12.) }],
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
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:03Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "dispatch",
                        "dispatch",
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
fn can_assign_dispatch_at_start() {
    let problem = create_problem_with_dispatch(Some(vec![VehicleDispatch {
        location: vec![0., 0.].to_loc(),
        limits: vec![VehicleDispatchLimit { max: 1, start: format_time(0.), end: format_time(2.) }],
        tag: None,
    }]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);

    let first_stop = &solution.tours[0].stops[0];
    assert_eq!(first_stop.time.arrival, format_time(0.));
    assert_eq!(first_stop.time.departure, format_time(2.));
    assert_eq!(first_stop.activities.len(), 2);
    assert_eq!(first_stop.activities[0].activity_type, "departure");
    assert_eq!(first_stop.activities[1].activity_type, "dispatch");
}

#[test]
fn can_handle_unassignable_dispatch() {
    let problem = create_problem_with_dispatch(Some(vec![VehicleDispatch {
        location: vec![1001., 0.].to_loc(),
        limits: vec![VehicleDispatchLimit { max: 1, start: format_time(10.), end: format_time(12.) }],
        tag: None,
    }]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.tours.is_empty());
    assert_eq!(solution.unassigned.map_or(0, |u| u.len()), 2);
}

parameterized_test! {can_handle_two_dispatch, (first_dispatch, second_dispatch, expected_location, expected_cost), {
    can_handle_two_dispatch_impl(first_dispatch, second_dispatch, expected_location, expected_cost);
}}

can_handle_two_dispatch! {
    case01: ((&[7., 0.], (7., 8.)), (&[8., 0.], (8., 9.)), &[7., 0.], 40.),
    case02: ((&[8., 0.], (8., 9.)), (&[7., 0.], (7., 8.)), &[7., 0.], 40.),
    case03: ((&[1001., 0.], (10., 11.)), (&[8., 0.], (8., 9.)), &[8., 0.], 44.),
}

fn can_handle_two_dispatch_impl(
    first_dispatch: (&[f64; 2], (f64, f64)),
    second_dispatch: (&[f64; 2], (f64, f64)),
    expected_location: &[f64; 2],
    expected_cost: f64,
) {
    let problem = create_problem_with_dispatch(Some(vec![
        VehicleDispatch {
            location: first_dispatch.0.to_vec().to_loc(),
            limits: vec![VehicleDispatchLimit {
                max: 1,
                start: format_time((first_dispatch.1).0),
                end: format_time((first_dispatch.1).0),
            }],
            tag: None,
        },
        VehicleDispatch {
            location: second_dispatch.0.to_vec().to_loc(),
            limits: vec![VehicleDispatchLimit {
                max: 1,
                start: format_time((second_dispatch.1).0),
                end: format_time((second_dispatch.1).0),
            }],
            tag: None,
        },
    ]));
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(!solution.tours.is_empty());
    assert_eq!(solution.tours[0].stops[1].location, expected_location.to_vec().to_loc());
    assert_eq!(solution.tours[0].stops[1].activities[0].activity_type, "dispatch");
    assert_eq!(solution.statistic.cost, expected_cost);
}

fn create_problem_with_dispatch_5jobs(vehicle_ids: Vec<&str>, dispatch: Option<Vec<VehicleDispatch>>) -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![2., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![2., 0.]),
                create_delivery_job("job4", vec![2., 0.]),
                create_delivery_job("job5", vec![2., 0.]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vehicle_ids.iter().map(|id| id.to_string()).collect(),
                shifts: vec![VehicleShift { dispatch: dispatch, ..create_default_vehicle_shift() }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    }
}

fn assert_tours(tours: &[Tour], values: (f64, f64, f64)) {
    tours.iter().for_each(|tour| {
        assert_eq!(tour.stops.len(), 4);

        assert_eq!(tour.stops[0].time.departure, format_time(values.0));
        assert_eq!(tour.stops[0].activities.len(), 1);
        assert_eq!(tour.stops[0].activities[0].activity_type, "departure");

        assert_eq!(tour.stops[1].activities.len(), 1);
        assert_eq!(tour.stops[1].activities[0].activity_type, "dispatch");
        assert_eq!(tour.stops[1].time.arrival, format_time(values.1));
        assert_eq!(tour.stops[1].time.departure, format_time(values.2));
    });
}

#[test]
fn can_dispatch_multiple_vehicles_at_single_dispatch() {
    let problem = create_problem_with_dispatch_5jobs(
        vec!["v1", "v2", "v3", "v4", "v5"],
        Some(vec![VehicleDispatch {
            location: vec![1., 0.].to_loc(),
            limits: vec![
                VehicleDispatchLimit { max: 2, start: format_time(10.), end: format_time(12.) },
                VehicleDispatchLimit { max: 3, start: format_time(13.), end: format_time(16.) },
            ],
            tag: None,
        }]),
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 5);

    assert_tours(&solution.tours[0..2], (9., 10., 12.));
    assert_tours(&solution.tours[2..5], (12., 13., 16.));
}
