use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::assert_vehicle_agnostic;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::solver::Builder;
use vrp_core::utils::Environment;

#[test]
fn can_use_init_solution_with_dispatch() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job_with_times("job2", vec![1., 0.], vec![(7, 100)], 1.),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1".to_string(), "v2".to_string()],
                shifts: vec![VehicleShift {
                    dispatch: Some(vec![VehicleDispatch {
                        location: vec![0., 0.].to_loc(),
                        limits: vec![
                            VehicleDispatchLimit { max: 1, start: format_time(2.), end: format_time(4.) },
                            VehicleDispatchLimit { max: 1, start: format_time(4.), end: format_time(6.) },
                        ],
                        tag: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let init_solution = Solution {
        statistic: Statistic {
            cost: 34.,
            distance: 4,
            duration: 10,
            times: Timing { driving: 4, serving: 6, waiting: 0, break_time: 0 },
        },
        tours: vec![
            Tour {
                vehicle_id: "v1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    Stop {
                        location: vec![0., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:00Z".to_string(),
                            departure: "1970-01-01T00:00:04Z".to_string(),
                        },
                        distance: 0,
                        load: vec![1],
                        activities: vec![
                            Activity {
                                job_id: "departure".to_string(),
                                activity_type: "departure".to_string(),
                                location: None,
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:00Z".to_string(),
                                    end: "1970-01-01T00:00:02Z".to_string(),
                                }),
                                job_tag: None,
                            },
                            Activity {
                                job_id: "dispatch".to_string(),
                                activity_type: "dispatch".to_string(),
                                location: None,
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:02Z".to_string(),
                                    end: "1970-01-01T00:00:04Z".to_string(),
                                }),
                                job_tag: None,
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        1,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:07Z"),
                        2,
                    ),
                ],
                statistic: Statistic {
                    cost: 17.,
                    distance: 2,
                    duration: 5,
                    times: Timing { driving: 2, serving: 3, waiting: 0, break_time: 0 },
                },
            },
            Tour {
                vehicle_id: "v2".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    Stop {
                        location: vec![0., 0.].to_loc(),
                        time: Schedule {
                            arrival: "1970-01-01T00:00:00Z".to_string(),
                            departure: "1970-01-01T00:00:06Z".to_string(),
                        },
                        distance: 0,
                        load: vec![1],
                        activities: vec![
                            Activity {
                                job_id: "departure".to_string(),
                                activity_type: "departure".to_string(),
                                location: None,
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:00Z".to_string(),
                                    end: "1970-01-01T00:00:04Z".to_string(),
                                }),
                                job_tag: None,
                            },
                            Activity {
                                job_id: "dispatch".to_string(),
                                activity_type: "dispatch".to_string(),
                                location: None,
                                time: Some(Interval {
                                    start: "1970-01-01T00:00:04Z".to_string(),
                                    end: "1970-01-01T00:00:06Z".to_string(),
                                }),
                                job_tag: None,
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        1,
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:09Z", "1970-01-01T00:00:09Z"),
                        2,
                    ),
                ],
                statistic: Statistic {
                    cost: 17.,
                    distance: 2,
                    duration: 5,
                    times: Timing { driving: 2, serving: 3, waiting: 0, break_time: 0 },
                },
            },
        ],
        ..create_empty_solution()
    };
    let environment = Arc::new(Environment::default());
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = Arc::new((problem.clone(), vec![matrix]).read_pragmatic().unwrap());
    let core_solution = to_core_solution(&init_solution, core_problem.clone(), environment.random.clone()).unwrap();

    let (core_solution, _, metrics) = Builder::new(core_problem.clone(), environment)
        .with_max_generations(Some(100))
        .with_init_solutions(vec![core_solution], None)
        .build()
        .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
        .solve()
        .unwrap_or_else(|err| panic!("cannot solve the problem: {}", err));
    let result_solution = create_solution(&core_problem, &core_solution, metrics.as_ref());

    assert_vehicle_agnostic(result_solution, init_solution);
}
