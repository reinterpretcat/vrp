use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

fn create_test_plan_with_three_jobs() -> Plan {
    Plan {
        jobs: vec![
            create_delivery_job_with_order("job1", (2., 0.), 2),
            create_delivery_job_with_order("job2", (5., 0.), 1),
            create_delivery_job("job3", (7., 0.)),
        ],
        ..create_empty_plan()
    }
}

fn create_test_limit() -> Option<VehicleLimits> {
    Some(VehicleLimits { max_distance: Some(15.), shift_time: None, tour_size: None, areas: None })
}

fn create_prioritized_objective() -> Vec<Vec<Objective>> {
    vec![
        vec![Objective::MinimizeUnassignedJobs { breaks: None }],
        vec![Objective::MinimizeTours {}],
        vec![Objective::TourOrder { is_constrained: true }],
        vec![Objective::MinimizeCost],
    ]
}

#[test]
fn can_follow_orders() {
    let problem = Problem {
        plan: create_test_plan_with_three_jobs(),
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_matrix_profiles() },
        objectives: Some(create_prioritized_objective()),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 53.,
                distance: 20,
                duration: 23,
                times: Timing { driving: 20, serving: 3, ..Timing::default() },
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
                        3,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (5., 0.),
                        2,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        5
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:09Z", "1970-01-01T00:00:10Z"),
                        8
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (7., 0.),
                        0,
                        ("1970-01-01T00:00:15Z", "1970-01-01T00:00:16Z"),
                        13
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:23Z", "1970-01-01T00:00:23Z"),
                        20
                    )
                ],
                statistic: Statistic {
                    cost: 53.,
                    distance: 20,
                    duration: 23,
                    times: Timing { driving: 20, serving: 3, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_assign_more_jobs_ignoring_order_with_default_objective() {
    let problem = Problem {
        plan: create_test_plan_with_three_jobs(),
        fleet: Fleet {
            vehicles: vec![VehicleType { limits: create_test_limit(), ..create_default_vehicle_type() }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.statistic.distance, 14);
}

#[test]
fn can_follow_order_when_prioritized_property_set() {
    let problem = Problem {
        plan: create_test_plan_with_three_jobs(),
        fleet: Fleet {
            vehicles: vec![VehicleType { limits: create_test_limit(), ..create_default_vehicle_type() }],
            profiles: create_default_matrix_profiles(),
        },
        objectives: Some(create_prioritized_objective()),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(
        solution.unassigned,
        Some(vec![UnassignedJob {
            job_id: "job3".to_string(),
            reasons: vec![UnassignedJobReason {
                code: "MAX_DISTANCE_CONSTRAINT".to_string(),
                description: "cannot be assigned due to max distance constraint of vehicle".to_string(),
                detail: Some(UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }),
            }]
        }])
    );
}

#[test]
fn can_handle_order_between_special_activities() {
    let create_test_job = |id: &str, location: (f64, f64), order: i32| Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { times: None, location: location.to_loc(), duration: 100., tag: None }],
            demand: Some(vec![1]),
            order: Some(order),
        }]),
        ..create_job(id)
    };
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_test_job("job1", (1., 0.), 2), create_test_job("job2", (2., 0.), 1)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(1000.).to_string(),
                        location: (10., 0.).to_loc(),
                    }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(100.), format_time(200.)]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 1.,
                            location: Some((0., 0.).to_loc()),
                            tag: None,
                        }],
                        policy: Some(VehicleOptionalBreakPolicy::SkipIfNoIntersection),
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        get_ids_from_tour(&solution.tours[0]),
        vec![vec!["departure"], vec!["job2"], vec!["break"], vec!["job1"], vec!["arrival"]]
    );
}
