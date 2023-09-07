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
    Some(VehicleLimits { max_distance: Some(15.), max_duration: None, tour_size: None })
}

fn create_order_objective(is_constrained: bool) -> Vec<Vec<Objective>> {
    if is_constrained {
        vec![
            vec![Objective::MinimizeUnassignedJobs { breaks: None }],
            vec![Objective::MinimizeTours],
            vec![Objective::MinimizeCost],
        ]
    } else {
        vec![
            vec![Objective::MinimizeUnassignedJobs { breaks: None }],
            vec![Objective::MinimizeTours],
            vec![Objective::TourOrder],
            vec![Objective::MinimizeCost],
        ]
    }
}

fn create_problem(is_constrained: bool, limits: Option<VehicleLimits>) -> Problem {
    Problem {
        plan: create_test_plan_with_three_jobs(),
        fleet: Fleet {
            vehicles: vec![VehicleType { limits, ..create_default_vehicle_type() }],
            ..create_default_fleet()
        },
        objectives: Some(create_order_objective(is_constrained)),
        ..create_empty_problem()
    }
}

#[test]
fn can_follow_orders() {
    let is_constrained = true;
    let problem = create_problem(is_constrained, None);
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![3])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![2])
                            .distance(5)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(9., 10.)
                            .load(vec![1])
                            .distance(8)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((7., 0.))
                            .schedule_stamp(15., 16.)
                            .load(vec![0])
                            .distance(13)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(23., 23.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(3).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_assign_more_jobs_ignoring_order_when_is_not_constrained() {
    let is_constrained = false;
    let problem = create_problem(is_constrained, create_test_limit());
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.statistic.distance, 14);
}

#[test]
fn can_follow_order_with_default_objective() {
    let problem = Problem { objectives: None, ..create_problem(false, create_test_limit()) };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_some());
    assert_eq!(solution.statistic.distance, 10);
}

#[test]
fn can_follow_order_when_prioritized_property_set() {
    let is_constrained = true;
    let problem = create_problem(is_constrained, create_test_limit());
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(
        solution.unassigned,
        Some(vec![UnassignedJob {
            job_id: "job3".to_string(),
            reasons: vec![UnassignedJobReason {
                code: "TOUR_ORDER_CONSTRAINT".to_string(),
                description: "cannot be assigned due to tour order constraint".to_string(),
                details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
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
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (10., 0.).to_loc() }),
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
            ..create_default_fleet()
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
