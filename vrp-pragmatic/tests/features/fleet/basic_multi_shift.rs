use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;
use crate::{format_time, parse_time};
use vrp_core::utils::compare_floats;

#[test]
fn can_use_multiple_times_from_vehicle_and_job() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job2", (10., 0.), vec![(100, 200)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![
                    VehicleShift {
                        start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                        end: Some(ShiftEnd { earliest: None, latest: format_time(99.), location: (0., 0.).to_loc() }),
                        ..create_default_vehicle_shift()
                    },
                    VehicleShift {
                        start: ShiftStart { earliest: format_time(100.), latest: None, location: (0., 0.).to_loc() },
                        end: Some(ShiftEnd { earliest: None, latest: format_time(200.), location: (0., 0.).to_loc() }),
                        ..create_default_vehicle_shift()
                    },
                ],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_vehicle_agnostic(
        solution,
        Solution {
            statistic: Statistic {
                cost: 102.,
                distance: 40,
                duration: 42,
                times: Timing { driving: 40, serving: 2, ..Timing::default() },
            },
            tours: vec![
                Tour {
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
                            0,
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (10., 0.),
                            0,
                            ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                            10,
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:21Z", "1970-01-01T00:00:21Z"),
                            20,
                        ),
                    ],
                    statistic: Statistic {
                        cost: 51.,
                        distance: 20,
                        duration: 21,
                        times: Timing { driving: 20, serving: 1, ..Timing::default() },
                    },
                },
                Tour {
                    vehicle_id: "my_vehicle_1".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 1,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            1,
                            ("1970-01-01T00:01:40Z", "1970-01-01T00:01:40Z"),
                            0,
                        ),
                        create_stop_with_activity(
                            "job2",
                            "delivery",
                            (10., 0.),
                            0,
                            ("1970-01-01T00:01:50Z", "1970-01-01T00:01:51Z"),
                            10,
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:02:01Z", "1970-01-01T00:02:01Z"),
                            20,
                        ),
                    ],
                    statistic: Statistic {
                        cost: 51.,
                        distance: 20,
                        duration: 21,
                        times: Timing { driving: 20, serving: 1, ..Timing::default() },
                    },
                },
            ],
            ..create_empty_solution()
        },
    );
}

#[test]
fn can_prefer_first_days_with_minimize_arrival_time_objective() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
            ..create_empty_plan()
        },
        objectives: Some(vec![
            vec![Objective::MinimizeUnassignedJobs { breaks: None }],
            vec![Objective::MinimizeArrivalTime],
            vec![Objective::MinimizeCost],
        ]),
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![0., 100., 200., 300., 400.]
                    .iter()
                    .map(|earliest| VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(*earliest),
                            latest: None,
                            location: (0., 0.).to_loc(),
                        },
                        end: None,
                        ..create_default_vehicle_shift()
                    })
                    .collect(),
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let mut departures = solution
        .tours
        .iter()
        .filter_map(|tour| tour.stops.first())
        .map(|stop| parse_time(&stop.schedule().departure))
        .collect::<Vec<_>>();
    departures.sort_by(|a, b| compare_floats(*a, *b));
    assert_eq!(departures, vec![0., 100.]);
}
